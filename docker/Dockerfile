FROM ubuntu:20.04

ENV DEBIAN_FRONTEND noninteractive

RUN apt update -y && \
    apt install -y build-essential git sudo python3 wget curl neovim htop \
	ruby bison cmake

ARG USER_UID=1000
ARG USER_GID=1000

#Enable sudo group
RUN echo "%sudo ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers
ADD entrypoint.sh "/usr/bin/"
WORKDIR /tmp


#Create user "user"
RUN groupadd -g ${USER_GID} user
RUN useradd -l --shell /bin/bash -c "" -m -u ${USER_UID} -g user -G sudo user
WORKDIR "/home/user"
USER user

RUN wget -O ~/.gdbinit-gef.py -q https://gef.blah.cat/sh \
  && echo source ~/.gdbinit-gef.py >> ~/.gdbinit


#################################################################
# Clone Aurora repository and prepare environment
RUN git clone https://github.com/RUB-SysSec/aurora

RUN mkdir evaluation

ENV AURORA_GIT_DIR=/home/user/aurora
ENV EVAL_DIR=/home/user/evaluation
ENV AFL_DIR=$EVAL_DIR/afl-fuzz
ENV AFL_WORKDIR=$EVAL_DIR/afl-workdir

WORKDIR $EVAL_DIR
RUN mkdir -p $EVAL_DIR/inputs/crashes && mkdir -p $EVAL_DIR/inputs/non_crashes

#################################################################
# Install AFL
RUN wget -q -c https://lcamtuf.coredump.cx/afl/releases/afl-latest.tgz && tar xf afl-latest.tgz && mv afl-2.52b afl-fuzz

##  apply patch & build
RUN cd afl-fuzz && patch -p1 < ${AURORA_GIT_DIR}/crash_exploration/crash_exploration.patch && make -j

#################################################################
# Build mruby target
RUN git clone https://github.com/mruby/mruby.git && cd mruby && git checkout 88604e39ac9c25ffdad2e3f03be26516fe866038
## build afl version
RUN CC=$AFL_DIR/afl-gcc printenv | grep CC
RUN cd mruby && CC=$AFL_DIR/afl-gcc make -e -j && mv ./bin/mruby ../mruby_fuzz

# clean
RUN cd mruby && make clean

# build normal version for tracing/rca
RUN cd mruby && CFLAGS="-ggdb -O0" make -e -j && mv ./bin/mruby ../mruby_trace

#################################################################
# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -q -y --default-toolchain nightly
ENV PATH="/home/user/.cargo/bin:${PATH}"


#################################################################
# Install Pin
RUN wget -q -c http://software.intel.com/sites/landingpage/pintool/downloads/pin-3.15-98253-gb56e429b1-gcc-linux.tar.gz && tar -xzf pin*.tar.gz
ENV PIN_ROOT="/home/user/evaluation/pin-3.15-98253-gb56e429b1-gcc-linux"

RUN mkdir -p "${PIN_ROOT}/source/tools/AuroraTracer" && cp -r ${AURORA_GIT_DIR}/tracing/* ${PIN_ROOT}/source/tools/AuroraTracer

## requires PIN_ROOT to be set correctly
RUN cd ${PIN_ROOT}/source/tools/AuroraTracer && make obj-intel64/aurora_tracer.so


#################################################################
# Complete setting up evaluation directory
RUN cp $AURORA_GIT_DIR/example.zip $EVAL_DIR && unzip -q example.zip && cp -r example/mruby_type_confusion/seed .
RUN echo "@@" > arguments.txt

WORKDIR /home/user
