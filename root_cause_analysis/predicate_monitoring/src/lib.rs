use log::{debug, error, info, trace, warn};
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use predicate::*;
use ptracer::{ContinueMode, Ptracer};
use register::*;
use rflags::RFlags;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use trace_analysis::predicates::SerializedPredicate;
use zydis::*;

mod predicate;
mod register;
mod rflags;

fn new_decoder() -> Decoder {
    Decoder::new(MachineMode::LONG_64, AddressWidth::_64).expect("failed to create decoder")
}

fn instruction(decoder: &Decoder, pid: Pid, address: usize) -> Option<DecodedInstruction> {
    let mut code = [0u8; 16];
    ptracer::util::read_data(pid, address, &mut code).expect("failed to read memory");
    trace!("code = {:02x?}", &code);

    let instruction = decoder.decode(&code).expect("failed to decode instruction");
    if let Some(instruction) = instruction {
        Some(instruction)
    } else {
        warn!("No instructions found at {:#018x}.", address);
        None
    }
}

fn disasm(log_level: log::Level, decoder: &Decoder, pid: Pid, address: usize, length: usize) {
    debug!("disasm at {:#018x} with length {}:", address, length);

    let formatter = Formatter::new(FormatterStyle::INTEL).expect("failed to create formatter");
    let mut code = vec![0u8; length];
    if let Err(err) = ptracer::util::read_data(pid, address, &mut code) {
        warn!("failed to read memory for disasm, skipping: {:?}", err);
        return;
    }

    let mut buffer = [0u8; 200];
    let mut buffer = OutputBuffer::new(&mut buffer[..]);

    for (instruction, ip) in decoder.instruction_iterator(&code, address as u64) {
        formatter
            .format_instruction(&instruction, &mut buffer, Some(ip), None)
            .expect("failed to format instruction");
        log::log!(log_level, "{:#018x} {}", ip, buffer);
    }
}

#[derive(Debug, Clone)]
pub struct RootCauseCandidate {
    pub address: usize,
    pub score: f64,
    pub predicate: Predicate,
}

impl RootCauseCandidate {
    pub fn satisfied(
        &self,
        dbg: &mut Ptracer,
        old_registers: &nix::libc::user_regs_struct,
    ) -> nix::Result<bool> {
        let old_rip = old_registers.rip;
        let new_rip = dbg.registers.rip;
        debug!("old_rip = {:#018x}, new_rip = {:#018x}", old_rip, new_rip);
        let rflags = RFlags::from_bits_truncate(dbg.registers.eflags);
        trace!("rflags = {:#018x}", rflags);

        // disasm
        if log::log_enabled!(log::Level::Trace) {
            let decoder = new_decoder();
            disasm(
                log::Level::Trace,
                &decoder,
                dbg.event().pid().expect("pid missing"),
                old_rip as usize,
                32,
            );
            disasm(
                log::Level::Trace,
                &decoder,
                dbg.event().pid().expect("pid missing"),
                new_rip as usize,
                32,
            );
        }

        match self.predicate {
            Predicate::Compare(ref compare) => {
                let value = match compare.destination {
                    ValueDestination::Register(ref reg) => reg.value(&dbg.registers),
                    ValueDestination::Address(ref mem) => mem.address(&old_registers),
                    ValueDestination::Memory(ref access_size, ref mem) => {
                        let address = mem.address(&old_registers);
                        debug!("address = {:#018x}", address);

                        let value = ptracer::read(
                            dbg.event().pid().expect("pid missing"),
                            address as nix::sys::ptrace::AddressType,
                        )
                        .expect("failed to read memory value")
                            as usize;
                        debug!("raw value = {:#018x}", value);

                        match 1usize.checked_shl(*access_size as u32) {
                            Some(mask) => value & mask,
                            _ => value,
                        }
                    }
                };
                debug!(
                    "value = {:#018x}, compare.value = {:#018x}",
                    value, compare.value
                );

                Ok(match compare.compare {
                    Compare::Equal => value == compare.value,
                    Compare::Greater => value > compare.value,
                    Compare::GreaterOrEqual => value >= compare.value,
                    Compare::Less => value < compare.value,
                    Compare::NotEqual => value != compare.value,
                })
            }
            Predicate::Edge(ref edge) => match edge.transition {
                EdgeTransition::Taken => {
                    Ok(old_rip as usize == edge.source && new_rip as usize == edge.destination)
                }
                EdgeTransition::NotTaken => {
                    Ok(old_rip as usize == edge.source && new_rip as usize != edge.destination)
                }
            },
            Predicate::Visited => Ok(true),
            Predicate::FlagSet(flag) => Ok(rflags.contains(flag)),
        }
    }
}

fn convert_predicates(
    decoder: &Decoder,
    dbg: &mut Ptracer,
    predicates: Vec<SerializedPredicate>,
) -> HashMap<usize, RootCauseCandidate> {
    predicates
        .into_iter()
        .map(|pred| {
            debug!("pred = {:?}", pred);

            let address = pred.address;
            let instr =
                instruction(&decoder, dbg.pid, address).expect("failed to parse instruction");

            if log::log_enabled!(log::Level::Debug) {
                let formatter =
                    Formatter::new(FormatterStyle::INTEL).expect("failed to create formatter");
                let mut buffer = [0u8; 200];
                let mut buffer = OutputBuffer::new(&mut buffer[..]);

                formatter
                    .format_instruction(&instr, &mut buffer, Some(dbg.registers.rip), None)
                    .expect("failed to format instruction");
                println!("{:#018x} {}", dbg.registers.rip, buffer);
            }
            trace!("{:#018x?} -> {:?}", address, instr);

            let converted = predicate::convert_predicate(&pred.name, instr).and_then(|predicate| {
                Some(RootCauseCandidate {
                    address,
                    score: pred.score,
                    predicate,
                })
            });

            if converted.is_none() {
                warn!("could not convert predicate {:016x?}", pred);
            }

            converted
        })
        .filter_map(|pred| pred)
        .map(|pred| (pred.address, pred))
        .collect()
}

fn insert_breakpoints(dbg: &mut Ptracer, rccs: &HashMap<usize, RootCauseCandidate>) {
    for address in rccs.keys() {
        dbg.insert_breakpoint(*address)
            .expect("failed to insert breakpoint");
    }
    debug!("breakpoints = {:#018x?}", dbg.breakpoints());
}

fn add_rccs_single_steps(
    pid: Pid,
    dbg: &mut Ptracer,
    rccs: &HashMap<usize, RootCauseCandidate>,
    single_steping: &mut HashMap<Pid, nix::libc::user_regs_struct>,
) {
    let rip = dbg.registers.rip;

    if let Some(rcc) = rccs.get(&(dbg.registers.rip as usize)) {
        debug!(
            "breakpoint at {:#018x} of predicate {:016x?} reached",
            rip, rcc.predicate
        );

        single_steping.insert(pid, dbg.registers);
    }
}

fn check_rccs(
    dbg: &mut Ptracer,
    old_registers: &nix::libc::user_regs_struct,
    rccs: &mut HashMap<usize, RootCauseCandidate>,
    satisfaction: &mut Vec<(usize, Predicate)>,
) {
    let old_rip = old_registers.rip;
    let remove_breakpoint = |dbg: &mut Ptracer, address| {
        if let Err(err) = dbg.remove_breakpoint(address) {
            error!(
                "failed to remove breakpoint at {:#018x}, skipping: {:?}",
                address, err
            );
        }
    };

    if let Some(rcc) = rccs.get(&(old_rip as usize)) {
        debug!(
            "single step target at {:#018x} of predicate {:016x?} reached",
            dbg.registers.rip, rcc.predicate
        );

        if !rcc
            .satisfied(dbg, old_registers)
            .expect("failed to test predicate")
        {
            trace!("predicate {:016x?} NOT satisfied", rcc.predicate);
            return;
        }
    } else {
        // removing the breakpoint may have failed early when the predicate was satisfied
        remove_breakpoint(dbg, old_rip as usize);
        return;
    }

    // predicate satisfied
    if let Some(rcc) = rccs.remove(&(old_rip as usize)) {
        info!(
            "predicate {:016x?} satisfied, moving predicate to satisfaction and removing breakpoint",
            rcc.predicate
        );
        satisfaction.push((rcc.address, rcc.predicate));
        remove_breakpoint(dbg, rcc.address);
    }
}

fn collect_satisfied(
    decoder: &Decoder,
    dbg: &mut Ptracer,
    rccs: &mut HashMap<usize, RootCauseCandidate>,
    timeout: u64,
) -> Vec<(usize, Predicate)> {
    let mut satisfaction = vec![];
    let mut single_steping = HashMap::new();
    let start_time = Instant::now();

    loop {
        trace!("threads = {:?}", dbg.threads);
        trace!("registers = {:#018x?}", dbg.registers);
        trace!("single_steping = {:#018x?}", single_steping);

        let rip = dbg.registers.rip;
        trace!("rip = {:#018x}", rip);

        if let Some(pid) = dbg.event().pid() {
            if log::log_enabled!(log::Level::Trace) {
                disasm(log::Level::Trace, decoder, pid, rip as usize, 32);
            }

            // we assume that single stepping on a breakpoint raises two ptrace events
            // therefor when our thread is in single step mode
            // (even when hitting another breakpoint)
            // we can just check the rcc without checking the next breakpoint
            // otherwise we hit a breakpoint and need to request single stepping
            if let Some(old_registers) = single_steping.remove(&pid) {
                // handle previous single steping request
                check_rccs(dbg, &old_registers, rccs, &mut satisfaction)
            } else {
                // add single stepping request
                add_rccs_single_steps(pid, dbg, rccs, &mut single_steping);
            }
        }

        if start_time.elapsed().as_secs() >= timeout {
            info!("timeout reached, end debugging.");
            break;
        }

        // A ptrace call may return `ESRCH` when the debugee is
        // dead or not ptrace-stopped. Only a dead debugee is fatal.
        // Retry the request up to 3 times to verify the debugee is dead.
        let mut result = Ok(());
        for _ in 0..3 {
            // continue / single step debugee
            result = if single_steping.is_empty() {
                dbg.cont(ContinueMode::Default)
            } else {
                dbg.step(ContinueMode::Default)
            };

            // retry on ESRCH
            match result {
                Ok(_) => break,
                Err(err) => {
                    debug!("ptrace returned error: {}", err);

                    if err.as_errno() != Some(nix::errno::Errno::ESRCH) {
                        break;
                    }
                }
            }
        }
        let event = dbg.event();

        // handle unexpected missing debugee
        if let Err(err) = result {
            info!("event = {:?}", dbg.event());
            warn!(
                "debugee exited unexpected, cannot continue debugging: {:?}",
                err
            );
            break;
        } else {
            debug!("event = {:?}", dbg.event());
        }

        // handle exited / signaled debugee
        match event {
            WaitStatus::Exited(pid, ret) if *pid == dbg.pid => {
                info!(
                    "debugee exited graceful with return code {}, stopping.",
                    ret
                );
                break;
            }
            WaitStatus::Signaled(pid, signal, _) if *pid == dbg.pid => {
                info!(
                    "debugee exited ungraceful with signal {}, stopping.",
                    signal
                );
                break;
            }
            _ => {}
        }

        if dbg.threads.is_empty() {
            info!("no more threads, end debugging.");
            break;
        }
    }

    info!("satisfaction = {:#018x?}", satisfaction);

    satisfaction
}

pub fn rank_predicates(
    mut dbg: Ptracer,
    predicates: Vec<SerializedPredicate>,
    timeout: u64,
) -> Vec<usize> {
    let decoder = new_decoder();

    let mut rccs = convert_predicates(&decoder, &mut dbg, predicates);
    debug!("rccs = {:#018x?}", rccs);

    insert_breakpoints(&mut dbg, &rccs);

    let satisfaction = collect_satisfied(&decoder, &mut dbg, &mut rccs, timeout);
    satisfaction.into_iter().map(|(addr, _)| addr).collect()
}

pub fn spawn_dbg(path: &Path, args: &[String]) -> Ptracer {
    Ptracer::spawn(path, args).expect("spawn failed")
}
