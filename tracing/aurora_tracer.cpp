/*BEGIN_LEGAL 
Intel Open Source License 

Copyright (c) 2002-2018 Intel Corporation. All rights reserved.
 
Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

Redistributions of source code must retain the above copyright notice,
this list of conditions and the following disclaimer.  Redistributions
in binary form must reproduce the above copyright notice, this list of
conditions and the following disclaimer in the documentation and/or
other materials provided with the distribution.  Neither the name of
the Intel Corporation nor the names of its contributors may be used to
endorse or promote products derived from this software without
specific prior written permission.
 
THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE INTEL OR
ITS CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
END_LEGAL */
#include <stdio.h>
#include <fstream>
#include <set>
#include <vector>
#include <iterator>
#include "pin.H"

#define NUM_REGS 23

enum EdgeType {Direct, Indirect, Conditional, Syscall, Return, Regular, Unknown};
static const std::string EDGE_TYPE_STR[7] = {
    "Direct", "Indirect", "Conditional", "Syscall", "Return", "Regular", "Unknown"
};

struct MemoryField {
    UINT64 address;
    UINT32 size;
    UINT64 value;

    std::string to_string() const {
        std::ostringstream ss;
        ss << "{\"address\":" << address << ",";
        ss << "\"size\":" << 8*size << ",";
        ss << "\"value\":" << value << "}";
        return ss.str();
    }
};

struct MemoryData {
    MemoryField last_addr  = {0, 0, 0};
    MemoryField min_addr   = {UINT64_MAX, 0, 0};
    MemoryField max_addr   = {0, 0, 0};
    MemoryField last_value = {0, 0, 0};
    MemoryField min_value  = {0, 0, UINT64_MAX};
    MemoryField max_value  = {0, 0, 0};

    std::string to_string() const {
        std::ostringstream ss;
        ss << "{\"last_address\":" << last_addr.address << ",";
        ss << "\"min_address\":"   << min_addr.address  << ",";
        ss << "\"max_address\":"   << max_addr.address  << ",";
        ss << "\"last_value\":" << last_value.value << ",";
        ss << "\"min_value\":"   << min_value.value  << ",";
        ss << "\"max_value\":"   << max_value.value  << "}";
        return ss.str();
    }
};

struct Value {
    bool is_set;
    UINT64 value;
};

struct InstructionData {
  UINT64 count;
  std::string disas;
  Value min_val[23];
  Value max_val[23];
  Value last_val[23];
  MemoryData mem;
  ADDRINT next_ins_addr; // note, in JSON this is called last_successor
};

static const REG REGISTERS[NUM_REGS] = {
    REG_RAX, REG_RBX, REG_RCX, REG_RDX, REG_RSI, REG_RDI, REG_RBP, REG_RSP,
    REG_R8, REG_R9, REG_R10, REG_R11, REG_R12, REG_R13, REG_R14, REG_R15,
    REG_SEG_CS, REG_SEG_SS, REG_SEG_DS, REG_SEG_ES, REG_SEG_FS, REG_SEG_GS, REG_GFLAGS
};
static const std::string REG_NAMES[NUM_REGS] = {
    "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp",
    "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
    "seg_cs", "seg_ss", "seg_ds", "seg_es", "seg_fs", "seg_gs", "eflags"
};

static FILE * g_trace_file;
static std::map<ADDRINT, InstructionData> g_instruction_map;
static std::map<std::pair<ADDRINT, ADDRINT>, std::pair<EdgeType, UINT64>> g_edge_map;
static EdgeType g_prev_ins_edge_type = Unknown;
static ADDRINT g_prev_ins_addr = 0;
static ADDRINT g_load_offset;
static ADDRINT g_low_address;
static ADDRINT g_first_ins_addr;
static UINT64 g_reg_state[23] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
static PIN_LOCK g_lock;


/**
 *  Add instruction to global instruction map
 */
VOID add_instruction(ADDRINT ins_addr, const std::string& ins_disas) {
    g_instruction_map[ins_addr] = {
            0,
            std::string(ins_disas), // disas is helpful in case something goes wrong
            {
                {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX},
                {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX},
                {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX},
                {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX},
                {0, UINT64_MAX}, {0, UINT64_MAX}, {0, UINT64_MAX}
            },
            {{0}},
            {{0}},
            {{0, 0, 0}, {UINT64_MAX, 0, 0}, {0, 0, 0}, {0, 0, 0}, {0, 0, UINT64_MAX}, {0, 0, 0}},
            0,
        };
}


/**
 * Add new edge to global edge map (if necessary) and increase visited count
 */
VOID ins_save_edge(ADDRINT predecessor, ADDRINT successor, EdgeType type) {
    std::pair<ADDRINT, ADDRINT> current_edge(predecessor, successor);
    if (g_edge_map.find(current_edge) == g_edge_map.end()) {
        g_edge_map[current_edge] = {/* type = */ type, /* visited count = */ 0};
    }
    else if (g_edge_map[current_edge].first != type) {
        LOG("[E] Edge(" + StringFromAddrint(predecessor) + ", " + StringFromAddrint(successor)
            + ") type differs\n");
        assert(g_edge_map[current_edge].first == type);
    }
    g_edge_map[current_edge].second += 1;
    // annotate previous instruction with the last successor node
    g_instruction_map[predecessor].next_ins_addr = successor;
}


/**
 * Update globally tracked register state and append written or modified values to address
 */
VOID update_reg_state(ADDRINT ins_addr, const CONTEXT * ctxt, const std::set<REG> * reg_ops) {
    PIN_REGISTER temp;
    InstructionData * tuple = &g_instruction_map[ins_addr];
    for (UINT32 i = 0; i < NUM_REGS; i++) {
        PIN_GetContextRegval(ctxt, REGISTERS[i], reinterpret_cast<UINT8*>(&temp));
        // check if reg value changed OR reg is register operand that is written to
        if (*(temp.qword) != g_reg_state[i] || std::find(reg_ops->begin(), reg_ops->end(), REGISTERS[i]) != reg_ops->end()) {
            g_reg_state[i] = *(temp.qword);
            if (tuple->min_val[i].value >= *(temp.qword)) tuple->min_val[i].value = *(temp.qword);
            if (tuple->max_val[i].value <= *(temp.qword)) tuple->max_val[i].value = *(temp.qword);
            // store "last-seen" value
            tuple->last_val[i].value = *(temp.qword);
            tuple->min_val[i].is_set  = true;
            tuple->max_val[i].is_set  = true;
            tuple->last_val[i].is_set = true;
        }
    }
}


/**
 * Update register state, save edge, and update global information based on current instruction
 */
VOID ins_save_state(ADDRINT ins_addr, const std::string& ins_disas, const CONTEXT * ctxt, const std::set<REG> * reg_ops, EdgeType type) {
    PIN_GetLock(&g_lock, ins_addr);
    // if first occurence, add instruction to map
    if (g_instruction_map.find(ins_addr) == g_instruction_map.end()) add_instruction(ins_addr, ins_disas);
    // increase visited count
    g_instruction_map[ins_addr].count += 1;
    // update which registers where changed during execution of the instruction
    update_reg_state(ins_addr, ctxt, reg_ops);
    // if a predecessor exists, save the edge
    if (g_prev_ins_addr) ins_save_edge(g_prev_ins_addr, ins_addr, g_prev_ins_edge_type);
    // data for next instruction to act upon
    g_prev_ins_addr = ins_addr;
    g_prev_ins_edge_type = type;
    PIN_ReleaseLock(&g_lock);
}

/**
 * read value from memory address (respects size)
 */
UINT64 read_from_addr(ADDRINT mem_addr, ADDRINT size, ADDRINT ins_addr) {
    switch(size) {
        case 1:
        {
            uint8_t * value = reinterpret_cast<uint8_t *>(mem_addr);
            return static_cast<uint64_t>(*value);
        }
        case 2:
        {
            uint16_t * value = reinterpret_cast<uint16_t *>(mem_addr);
            return static_cast<uint64_t>(*value);
        }
        case 4:
        {
            uint32_t * value = reinterpret_cast<uint32_t *>(mem_addr);
            return static_cast<uint64_t>(*value);
        }
        case 8:
        {
            uint64_t * value = reinterpret_cast<uint64_t *>(mem_addr);
            return static_cast<uint64_t>(*value);
        }
        default:
            LOG ("[E] Unhandled memory access size " + decstr(size) + " (" + decstr(size*8)
                 + " bits). Value set to 0 for " + StringFromAddrint(ins_addr) + "\n");
    }
    return 0;
}


VOID ins_save_memory_access(ADDRINT ins_addr, ADDRINT mem_addr, UINT32 size) {
    // Disregard everything with more than 8 bytes (we are not interested in floating point stuff)
    if (size > 8) {
        return;
    }
    PIN_GetLock(&g_lock, ins_addr);
    MemoryData* mem_data = &g_instruction_map[ins_addr].mem;
    if (mem_data->last_addr.size && mem_data->last_addr.size != size) {
        LOG("[E] Memory operand has different memory access sizes at " + StringFromAddrint(ins_addr) + "\n");
        assert(mem_data->last_addr.size == size && "Memory operand has different memory access sizes");
    }
    MemoryField access = {mem_addr, size, 0};
    access.value = read_from_addr(mem_addr, size, ins_addr);
    if (mem_data->max_addr.address <= access.address) mem_data->max_addr = access;
    if (mem_data->min_addr.address >= access.address) mem_data->min_addr = access;
    mem_data->last_addr = access;
    if (mem_data->max_value.value <= access.value) mem_data->max_value = access;
    if (mem_data->min_value.value >= access.value) mem_data->min_value = access;
    mem_data->last_value = access;
    PIN_ReleaseLock(&g_lock);
}


EdgeType get_edge_type(INS ins) {
    if (INS_IsRet(ins)) return Return;
    if (INS_IsCall(ins) || INS_IsBranch(ins)) {
        if (INS_Category(ins) == XED_CATEGORY_COND_BR) return Conditional;
        if (INS_IsIndirectControlFlow(ins)) return Indirect;
        if (INS_IsDirectControlFlow(ins)) return Direct;
        return Unknown;
    }
    if (INS_IsSyscall(ins)) return Syscall;
    return Regular;
}


std::set<REG>* get_written_reg_operands(INS ins) {
    std::set<REG>* reg_ops = new std::set<REG>();
    for (const REG& reg : REGISTERS) {
        if (INS_FullRegWContain(ins, reg)) reg_ops->insert(reg);
    }
    return reg_ops;
}


// Pin calls this function every time a new instruction is encountered
VOID Instruction(INS ins, VOID *v) {
    // Skip instructions outside main exec
    PIN_LockClient();
    const IMG image = IMG_FindByAddress(INS_Address(ins));
    PIN_UnlockClient();
    if (IMG_Valid(image) && IMG_IsMainExecutable(image)) {
        if (INS_IsHalt(ins)) {
            LOG("[W] Skipping instruction: " + StringFromAddrint(INS_Address(ins)) + " : "
                + INS_Disassemble(ins) + "\n");
            return;
        }
        std::set<REG>* reg_ops = get_written_reg_operands(ins);
        // Check whether the instruction is a branch | call | ret | ...
        EdgeType type = get_edge_type(ins);
        // For regular edges, put insertion point after execution else (calls/ret/(cond) branches) before
        IPOINT ipoint = (type == Regular ? IPOINT_AFTER : IPOINT_BEFORE);
        INS_InsertCall(ins,
            ipoint, (AFUNPTR)ins_save_state,
            IARG_ADDRINT, INS_Address(ins),
            IARG_PTR, new std::string(INS_Disassemble(ins)),
            IARG_CONST_CONTEXT,
            IARG_PTR, reg_ops,
            IARG_PTR, type,
            IARG_END
        );

        // Check whether we explicitly dereference memory
        if (!(INS_HasExplicitMemoryReference(ins) || INS_Stutters(ins)) || type != Regular) {
            return;
        }
        // Ignore non-typical operations such as vscatter/vgather
        if (!INS_IsStandardMemop(ins)) {
            LOG("[W] Non-standard memory operand encountered: " + StringFromAddrint(INS_Address(ins))
                + " : " + INS_Disassemble(ins) + "\n");
            return;
        }
        // Iterate over all memory operands of the instruction
        UINT32 mem_operands = INS_MemoryOperandCount(ins);
        for (UINT32 mem_op = 0; mem_op < mem_operands; mem_op++) {
            // Ensure that we can determine the size
            if (!INS_hasKnownMemorySize(ins)) {
                LOG("[W] Memory operand with unknown size encountered: " + StringFromAddrint(INS_Address(ins))
                    + " : " + INS_Disassemble(ins) + "\n");
                continue;
            }
            // Instrument only when we *write* to memory
            if (INS_MemoryOperandIsWritten(ins, mem_op)) {
                // Instrument only when the instruction is executed (conditional mov)
                INS_InsertPredicatedCall(
                    ins, IPOINT_AFTER, (AFUNPTR)ins_save_memory_access,
                    IARG_INST_PTR,
                    IARG_MEMORYOP_EA, mem_op,
                    IARG_MEMORYWRITE_SIZE,
                    IARG_END
                );
            }
        }
    }
}


VOID parse_maps() {
    FILE *fp;
    char line[2048];
    fp = fopen("/proc/self/maps", "r");
    if (fp == NULL) {
        LOG("[E] Failed to open /proc/self/maps");
        return;
    }
    while (fgets(line, 2048, fp) != NULL) {
        std::string s = std::string(line);
        if (strstr(line, "stack") != NULL) {
            std::string start = s.substr(0, s.find("-"));
            std::string end = s.substr(start.length() + 1, s.find(" ") - (start.length() + 1));
            LOG("[*] Stack: 0x" + start + " - 0x" + end + "\n");
        }
        if (strstr(line, "heap") != NULL) {
            std::string start = s.substr(0, s.find("-"));
            std::string end = s.substr(start.length() + 1, s.find(" ") - (start.length() + 1));
            LOG("[*] Heap: 0x" + start + " - 0x" + end + "\n");
        }
    }
    fclose(fp);
}


/**
 *  Extract metadata from main executable. Includes image base, load offset,
 *  first executed instruction address, and stack + heap ranges
 */
VOID parse_image(IMG img, VOID *v) {
    LOG("[+] Called parse_image on " + IMG_Name(img) + "\n");
    if (IMG_IsMainExecutable(img)) {
        g_load_offset  = IMG_LoadOffset(img);
        g_low_address  = IMG_LowAddress(img);
        LOG("[*] Image base: " + StringFromAddrint(g_low_address) + "\n");
        LOG("[*] Load offset: " + StringFromAddrint(g_load_offset) + "\n");
        ADDRINT img_entry_addr = IMG_EntryAddress(img);
        LOG("[*] Image entry address: " + StringFromAddrint(img_entry_addr) + "\n");
        g_first_ins_addr = g_load_offset + img_entry_addr;
        LOG("[*] First instruction address: " + StringFromAddrint(g_first_ins_addr) + "\n");
    }
}


/**
 *  Convert an array of REGISTER : data to JSON string
 */
std::string jsonify_reg_array(const Value* values) {
    std::ostringstream ss;
    for (int i = 0; i < 23; i++) {
        if (values[i].is_set) {
            ss << "\"" << i << "\":{\"name\":\"" << REG_NAMES[i] << "\",\"value\":" << values[i].value << "},";
        }
    }
    std::string str = ss.str();
    // remove last comma
    if (str.length() > 0)
        str.pop_back();
    return str;
}


/**
 *  Return a JSON representation as string of 'relevant' (sic) data
 */
std::string jsonify() {
    LOG("[+] Called jsonify\n");
    std::ostringstream ss;
    ss << "{\"image_base\":" << g_low_address;
    ss << ",\"first_address\":" << g_first_ins_addr;
    ss << ",\"last_address\":" << g_prev_ins_addr;
    ss << ",\"instructions\":[";
    bool first = true;
    for (auto const& ins : g_instruction_map){
        if (!first) ss << ",";
        first = false;
        if (ins.second.disas == "") {
            LOG("[E] Disassembly is empty for " + StringFromAddrint(ins.first) + "\n");
            assert(ins.second.disas != "" && "Disassembly is empty");
        }
        ss << "{\"address\":" << ins.first << ",\"mnemonic\":\"" << ins.second.disas << "\",\"registers_min\":{";
        ss << jsonify_reg_array(ins.second.min_val);
        ss << "},\"registers_max\":{";
        ss << jsonify_reg_array(ins.second.max_val);
        ss << "},\"registers_last\":{";
        ss << jsonify_reg_array(ins.second.last_val);
        ss << "},\"last_successor\":" << ins.second.next_ins_addr << ",";
        ss << "\"count\":" << ins.second.count;
        if (ins.second.mem.last_addr.size != 0) ss << "," << "\"memory\":" << ins.second.mem.to_string();
        ss << "}";
    }
    ss << "],\"edges\":[";
    first = true;
    for (auto const& edge : g_edge_map) {
        if (!first) ss << ",";
        first = false;
        // Convert edge type to str
        std::string edge_type_str = EDGE_TYPE_STR[static_cast<USIZE>(edge.second.first)];
        ss << "{\"from\":" << edge.first.first << ",\"to\":" << edge.first.second;
        ss << ",\"count\":" << edge.second.second;
        ss << ",\"edge_type\":\"" << edge_type_str << "\"}";
    }
    ss << "]}";
    return ss.str();
}


/**
 *  Write data as JSON to output file upon application exit
 */
VOID Fini(INT32 code, VOID *v) {
    LOG("[*] Last instruction: " + StringFromAddrint(g_prev_ins_addr) + "\n");
    std::string data = jsonify();
    fprintf(g_trace_file, "%s", data.c_str());
    fclose(g_trace_file);
    parse_maps();
    LOG("[=] Completed trace.\n");
}


// Allow renaming output file via -o switch
KNOB<std::string> KnobOutputFile(KNOB_MODE_WRITEONCE, "pintool",
    "o", "itrace.out", "specify output file name");


/* ===================================================================== */
/* Print Help Messages                                                   */
/* ===================================================================== */

INT32 Usage() {
    PIN_ERROR("This Pintool traces each instruction, dumping their addresses and additional state.\n"
              + KNOB_BASE::StringKnobSummary() + "\n");
    return -1;
}


INT32 Aslr() {
    PIN_ERROR("Disable ASLR before running this tool: echo 0 | sudo tee /proc/sys/kernel/randomize_va_space");
    return -1;
}


/* ===================================================================== */
/* Main                                                                  */
/* ===================================================================== */

int main(int argc, char * argv[]) {
    // Check if ASLR is disabled
    std::ifstream infile("/proc/sys/kernel/randomize_va_space");
    int aslr;
    if (!infile) {
        PIN_ERROR("Unable to check whether ASLR is enabled or not. Failed to open /proc/sys/kernel/randomize_va_space");
        return -1;
    }
    infile >> aslr;
    infile.close();
    if (aslr != 0) return Aslr();

    // Initialize pin
    if (PIN_Init(argc, argv)) return Usage();

    g_trace_file = fopen(KnobOutputFile.Value().c_str(), "w");


    // get image base address
    IMG_AddInstrumentFunction(parse_image, 0);

    // Register Instruction to be called to instrument instructions
    INS_AddInstrumentFunction(Instruction, 0);
    // Register Fini to be called when the application exits
    PIN_AddFiniFunction(Fini, 0);

    LOG("[*] Pintool: " + std::string(PIN_ToolFullPath()) + "\n");
    LOG("[*] Target:  " + std::string(PIN_VmFullPath()) + "\n");

    // Start the program, never returns
    PIN_StartProgram();

    return 0;
}

