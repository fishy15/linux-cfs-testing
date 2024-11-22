import gdb
import json

from dataclasses import asdict, dataclass, is_dataclass
from enum import Enum
from typing import List, Optional

#### TYPES ####

## All of the read functions here assume the string given
## represents either a pointer to the value or the actual
## value itself.

class CpuIdleType(Enum):
    CPU_IDLE = 0
    CPU_NOT_IDLE = 1
    CPU_NEWLY_IDLE = 2
    CPU_MAX_IDLE_TYPES = 3

def read_cpu_idle_type(idle):
    idle_value = read_value(idle)
    for option in CpuIdleType:
        if option.name == idle_value:
            return option
    raise Exception(f'No matching idle type for {idle_value}')

@dataclass
class SchedDomain:
    pass

def read_sched_domain(sd):
    print(sd)
    return SchedDomain()

@dataclass
class ReadyQueue:
    pass

def read_ready_queue(rq):
    print(rq)
    return ReadyQueue()

@dataclass
class CpuMask:
    mask: str

def read_cpumask(cpumask):
    output = exec_capture_output(f'p/t {cpumask}')
    bits = output.split()[4][1:-2]
    while len(bits) < CORES:
        bits = '0' + bits
    return CpuMask(bits)

@dataclass
class FBQType(Enum):
    regular = 0
    remote = 1
    all = 2

def read_fbq_type(fbq_type):
    fbq_type_value = read_value(fbq_type)
    for option in FBQType:
        if option.name == fbq_type_value:
            return option
    raise Exception(f'No matching fbq type for {fbq_type_value}')


@dataclass
class MigrationType(Enum):
    migrate_load = 0
    migrate_util = 1
    migrate_task = 2
    migrate_misfit = 3

def read_migration_type(migration_type):
    migration_type_value = read_value(migration_type)
    for option in MigrationType:
        if option.name == migration_type_value:
            return option
    raise Exception(f'No matching migration type for {migration_type_value}')


@dataclass
class LBEnv:
    sd: Optional[SchedDomain]
    src_rq: Optional[ReadyQueue]
    src_cpu: int
    dst_cpu: int
    dst_rq: Optional[ReadyQueue]
    dst_grpmask: CpuMask
    new_dst_cpu: int
    idle: CpuIdleType
    imbalance: int
    cpus: CpuMask
    flags: int
    loop: int
    loop_break: int
    loop_max: int
    fbq_type: FBQType
    migration_type: MigrationType
    # tasks: ListHead

def read_lb_env(lb_env) -> LBEnv:
    sd = read_if_not_null(f'{lb_env}.sd', read_sched_domain)
    src_rq = read_if_not_null(f'{lb_env}.src_rq', read_ready_queue)
    src_cpu = read_int(f'{lb_env}.src_cpu')
    dst_cpu = read_int(f'{lb_env}.dst_cpu')
    dst_rq = read_if_not_null(f'{lb_env}.dst_rq', read_ready_queue)
    dst_grpmask = read_if_not_null(f'{lb_env}.dst_grpmask', read_cpumask)
    new_dst_cpu = read_int(f'{lb_env}.new_dst_cpu')
    idle = read_cpu_idle_type(f'{lb_env}.idle')
    imbalance = read_int(f'{lb_env}.imbalance')
    cpus = read_if_not_null(f'{lb_env}.cpus', read_cpumask)
    flags = read_int(f'{lb_env}.flags')
    loop = read_int(f'{lb_env}.loop')
    loop_break = read_int(f'{lb_env}.loop_break')
    loop_max = read_int(f'{lb_env}.loop_max')
    fbq_type = read_fbq_type(f'{lb_env}.fbq_type')
    migration_type = read_migration_type(f'{lb_env}.migration_type')
    return LBEnv(sd, src_rq, src_cpu, dst_cpu, dst_rq, dst_grpmask, new_dst_cpu, idle,
            imbalance, cpus, flags, loop, loop_break, loop_max, fbq_type, migration_type)


@dataclass
class LBLogMsg:
    runs_load_balance: bool
    lb_env: LBEnv

def read_lb_logmsg(lb_logmsg) -> LBLogMsg:
    runs_load_balance = read_bool(f'{lb_logmsg}.runs_load_balance')
    if runs_load_balance:
        lb_env = read_lb_env(f'{lb_logmsg}.env')
    else:
        lb_env = None
    return LBLogMsg(runs_load_balance, lb_env)


@dataclass
class RDEntryLogMsg:
    max_newidle_lb_cost: int
    continue_balancing: int
    interval: int
    need_serialize: int
    lb_logmsg: LBLogMsg
    new_idle: CpuIdleType
    new_busy: int

def read_rebalance_domains_entry(entry) -> RDEntryLogMsg:
    max_newidle_lb_cost = read_int(f'{entry}.max_newidle_lb_cost')
    continue_balancing = read_int(f'{entry}.continue_balancing')
    interval = read_int(f'{entry}.interval')
    need_serialize = read_int(f'{entry}.need_serialize')
    lb_logmsg = read_lb_logmsg(f'{entry}.lb_logmsg')
    new_idle = read_cpu_idle_type(f'{entry}.new_idle')
    new_busy = read_int(f'{entry}.new_busy')
    return RDEntryLogMsg(max_newidle_lb_cost, continue_balancing, interval, need_serialize, lb_logmsg, 
            new_idle, new_busy)


@dataclass
class RDLogMsg:
    cpu: int
    idle: CpuIdleType
    sched_idle_cpu: int
    sd_buf: List[RDEntryLogMsg]

def read_rebalance_domains(rd_msg) -> RDLogMsg:
    cpu = read_int(f'{rd_msg}.cpu')
    idle = read_cpu_idle_type(f'{rd_msg}.idle')
    sched_idle_cpu = read_int(f'{rd_msg}.sched_idle_cpu')
    sd_buf = [read_rebalance_domains_entry(f'{rd_msg}.sd_buf[0]')]  # TODO: read more than one entry
    return RDLogMsg(cpu, idle, sched_idle_cpu, sd_buf)

class Encoder(json.JSONEncoder):
    def default(self, obj):
        print(obj)
        if isinstance(obj, Enum):
            print('case 1')
            return obj.name
        elif is_dataclass(obj):
            return asdict(obj)
        print(type(obj))
        return super().default(obj)

### Actual code ####

## Read in some information about cores based on the topology

if not TOPOLOGY:
    CORES=2
else:
    hyphen = TOPOLOGY.find('-')
    if hyphen != -1:
        CORES=int(TOPOLOGY[:hyphen])
    else:
        CORES=int(TOPOLOGY)

## setup file

try:
    FILE
except NameError:
    FILE = None

## set up iteration count
try:
    ITERS = int(ITERS)
except:
    ITERS = None

# print config to user
print('TOPOLOGY:', TOPOLOGY)
print('CORES:', CORES)
print('FILE:', FILE)
print('ITERS:', ITERS)

## Command to interact with GDB

def exec(cmd):
    print(cmd)
    gdb.execute(cmd)

def exec_capture_output(cmd):
    print(cmd)
    output = gdb.execute(cmd, False, True).strip()
    return output

## helper functions to extract information out 

def get_slot(rq, position=None):
    if position is None:
        position = f'{rq}->cfs.karan_logbuf.position'
    print('RQ:', read_value(rq))
    print('POSITION:', read_value(position))
    return read_value(f'{rq}->cfs.karan_logbuf.msgs[{position}]')

def get_logmsg_info(ptr):
    if ptr == '0x0' or read_value(ptr) == '0x0':
        return None
    ptr = f'((struct karan_logmsg *) {ptr})'  # cast to the right type
    codepath = read_value(f'{ptr}->codepath')
    print('CODEPATH:', codepath)
    if codepath == 'REBALANCE_DOMAINS':
        data = read_rebalance_domains(f'{ptr}->rd_msg')
    else:
        data = None
    return data
    '''
    if data is not None:
        print(data)
        total_data.append(data)
    '''

def read_value(val):
    tokens = exec_capture_output(f'p {val}').split()
    # if the type is given of a pointer, then return the value afterwards
    try:
        ptr_token = tokens.index('*)')
        return tokens[ptr_token+1]
    except ValueError:
        return tokens[2]

def read_int(val):
    return int(read_value(val))

def read_bool(val):
    # covers int and bool possibilities
    return read_value(val) not in ['false', 'False', '0']


def read_if_not_null(ptr, f):
    ptr_address = exec_capture_output(f'p {ptr}')
    if '0x0' not in ptr_address.split():
        return f(f'(*{ptr})')
    else:
        return None

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
    ptr = read_value('buf')
    
    if ptr == '0x0':
        print('it\'s null...')
        exec('c')
        return
    print(ptr)
    get_logmsg_info(ptr)

total_data = []
def handle_wraparound():
    global total_data
    # dump whole buffer
    buf = 'rq->cfs.karan_logbuf'
    num_entries = read_int(f'sizeof({buf}.msgs) / sizeof(*{buf}.msgs)')
    print('NUM ENTRIES:', num_entries)
    num_entries = 4
    data = [get_logmsg_info(get_slot('rq', i)) for i in range(num_entries)]
    total_data.extend(data)
    if FILE is not None:
        with open(FILE, 'w') as f:
            json.dump(total_data, f, cls=Encoder)
            print(len(total_data))
            # print(json.dumps(total_data, cls=Encoder))
    
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
        elif typ == 5:
            handle_wraparound() # dmup
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

exec('dis 2')
exec('dis 3')
exec('dis 4')
exec('b karan_msg_alloc_wraparound')

exec('c')

# go until the first example
if ITERS is not None:
    for i in range(ITERS):
        print('ON ITER', i)
        exec('c')
    json.dump(total_data, file, cls=Encoder)
    print(json.dumps(total_data, cls=Encoder))
    file.close()
