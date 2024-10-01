import gdb

from dataclasses import dataclass
from enum import Enum
from typing import List

#### TYPES ####

## All of the read functions here assume the string given
## represents either a pointer to the value or the actual
## value itself.

class CpuIdleType(Enum):
    CPU_IDLE = 0
    CPU_NOT_IDLE = 1
    CPU_NEWLY_IDLE = 2
    CPU_MAX_IDLE_TYPES = 3

# TODO: read it in
@dataclass
class LBEnv:
    pass

def read_lb_env(lb_env) -> LBEnv:
    return None


@dataclass
class LBLogMsg:
    lb_env: LBEnv

def read_lb_logmsg(lb_logmsg) -> LBLogMsg:
    lb_env = read_lb_env(f'{lb_logmsg}.lb_env')
    return LBLogMsg(lb_env)


@dataclass
class RDEntryLogMsg:
    max_newidle_lb_cost: int
    continue_balancing: int
    interval: int
    need_serialize: int
    runs_load_balance: int
    lb_logmsg: LBLogMsg
    new_idle: CpuIdleType
    new_busy: int

def read_rebalance_domains_entry(entry) -> RDEntryLogMsg:
    max_newidle_lb_cost = int(exec_capture_output(f'p {entry}.max_newidle_lb_cost', 2))
    continue_balancing = int(exec_capture_output(f'p {entry}.continue_balancing', 2))
    interval = int(exec_capture_output(f'p {entry}.interval', 2))
    need_serialize = int(exec_capture_output(f'p {entry}.need_serialize', 2))
    runs_load_balance = bool(exec_capture_output(f'p {entry}.interval', 2))
    lb_logmsg = read_lb_logmsg(f'{entry}.lb_logmsg')
    new_idle = exec_capture_output(f'p {entry}.new_idle', 2)
    new_busy = int(exec_capture_output(f'p {entry}.new_busy', 2))
    return RDEntryLogMsg(max_newidle_lb_cost, continue_balancing, interval, need_serialize, runs_load_balance,
            lb_logmsg, new_idle, new_busy)


@dataclass
class RDLogMsg:
    cpu: int
    idle: CpuIdleType
    sched_idle_cpu: int
    sd_buf: List[RDEntryLogMsg]

def read_rebalance_domains(rd_msg) -> RDLogMsg:
    cpu = int(exec_capture_output(f'p {rd_msg}.cpu', 2))
    idle = exec_capture_output(f'p {rd_msg}.idle', 2)
    sched_idle_cpu = int(exec_capture_output(f'p {rd_msg}.sched_idle_cpu', 2))
    sd_buf = [read_rebalance_domains_entry(f'{rd_msg}.sd_buf[0]')] # TODO: read more than one entry
    return RDLogMsg(cpu, idle, sched_idle_cpu, sd_buf)


### Actual code ####

## Read in some information about cores based on the topology

print('TOPOLOGY:', TOPOLOGY)
if not TOPOLOGY:
    CORES=2
else:
    hyphen = TOPOLOGY.find('-')
    if hyphen != -1:
        CORES=int(TOPOLOGY[:hyphen])
    else:
        CORES=int(TOPOLOGY)

print('CORES:', CORES)

## Command to interact with GDB

def exec(cmd):
    print(cmd)
    gdb.execute(cmd)

def exec_capture_output(cmd, arg=None):
    print(cmd)
    output = gdb.execute(cmd, False, True).strip()
    if arg is None:
        return output
    else:
        return output.split()[arg]

## helper functions to extract information out 

def get_logmsg_ptr(output):
    # looks like: "$1 = (struct karan_logmsg *) 0x0 <fixed_percpu_data>"
    return output.split()[5]

def get_slot(rq):
    position = f'{rq}->cfs.karan_logbuf.position'
    print('RQ:', exec_capture_output(f'p {rq}'))
    print('POSITION:', exec_capture_output(f'p {position}'))
    output = exec_capture_output(f'p {rq}->cfs.karan_logbuf.msgs[{position}]')
    return get_logmsg_ptr(output)

def get_codepath(ptr):
    codepath = exec_capture_output(f'p {ptr}->codepath')
    return codepath.split()[2]

def get_logmsg_info(ptr):
    ptr = f'((struct karan_logmsg *) {ptr})'
    codepath = get_codepath(ptr)
    if codepath == 'REBALANCE_DOMAINS':
        data = read_rebalance_domains(f'{ptr}.rd_msg')
        print(data)
    else:
        pass

## check that all cores have been readied

ready_count = 0
def handle_ready():
    global ready_count
    ready_count += 1

## handlers for different types of load balance callers

def handle_rebalance_domains():
    print('handling rebalance domains...')
    ptr = get_slot('rq')
    
    if ptr == '0x0':
        print('it\'s null...')
        exec('c')
        return
    print(ptr)
    get_logmsg_info(ptr)

def handle_newidle_balance():
    print('handling newidle balance...')
    ptr = get_slot('this_rq')
    
    if ptr == '0x0':
        print('it\'s null...')
        exec('c')
        return
    print(ptr)
    get_logmsg_info(ptr)

def handle_karan_newidle_balance_ret():
    print('handling karan_newidle_balance_ret...')
    ptr = get_logmsg_ptr(exec_capture_output('p buf'))
    
    if ptr == '0x0':
        print('it\'s null...')
        exec('c')
        return
    print(ptr)
    get_logmsg_info(ptr)

def breakpoint_handler(event):
    if isinstance(event, gdb.BreakpointEvent):
        bp = event.breakpoints[0]
        typ = bp.number

        if typ == 1:
            handle_ready()
        elif typ == 2:
            handle_rebalance_domains()
        elif typ == 3:
            handle_newidle_balance()
        elif typ == 4:
            handle_karan_newidle_balance_ret()
        else:
            print('oh no')

gdb.events.stop.connect(breakpoint_handler)

## actual script

# connect to remote
exec('file /l/kbuild/vmlinux')
exec('tar rem :1234')

exec('b karan_log_init:ready')

while ready_count < CORES:
    exec('c')

print('we are ready now')

# once we have made everything ready
exec('dis 1')
exec('b rebalance_domains:out')
exec('b newidle_balance:out')
exec('b karan_newidle_balance_ret')

# go until the first example
exec('c')
