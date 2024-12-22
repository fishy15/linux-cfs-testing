import gdb
import json

from dataclasses import asdict, dataclass, is_dataclass
from enum import Enum
from typing import List, Optional

import sys

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
class SWBPerCpuLogMsg:
    cpu_id: int
    idle_cpu: bool
    is_core_idle_cpu: bool

def read_swb_per_cpu_logmsg(per_cpu_msg) -> SWBPerCpuLogMsg:
    cpu_id = read_int(f'{per_cpu_msg}.cpu_id')
    idle_cpu = read_bool(f'{per_cpu_msg}.idle_cpu')
    is_core_idle_cpu = read_bool(f'{per_cpu_msg}.is_core_idle_cpu')
    return SWBPerCpuLogMsg(cpu_id, idle_cpu, is_core_idle_cpu)


@dataclass
class SWBLogMsg:
    swb_cpus: CpuMask
    dst_cpu: int
    cpus: CpuMask
    idle: Optional[int]
    dst_nr_running: Optional[int]
    dst_ttwu_pending: Optional[int]
    per_cpu_msgs: Optional[List[SWBPerCpuLogMsg]]
    group_balance_mask_sg: Optional[CpuMask]
    group_balance_cpu_sg: Optional[int]

def read_swb_logmsg(swb_logmsg) -> SWBLogMsg:
    swb_cpus = read_cpumask(f'{swb_logmsg}.swb_cpus')
    dst_cpu = read_int(f'{swb_logmsg}.dst_cpu')
    cpus = read_cpumask(f'{swb_logmsg}.cpus')

    went_forward = read_bool(f'{swb_logmsg}.went_forward')
    idle = read_int(f'{swb_logmsg}.idle') if went_forward else None
    
    next_two_checked = read_bool(f'{swb_logmsg}.next_two_checked')
    dst_nr_running = read_int(f'{swb_logmsg}.dst_nr_running') if next_two_checked else None
    dst_ttwu_pending = read_int(f'{swb_logmsg}.dst_ttwu_pending') if next_two_checked else None

    num_entries = read_int(f'{swb_logmsg}.next_per_cpu_msg_slot - {swb_logmsg}.per_cpu_msgs')
    print('NUM ENTRIES:', num_entries)
    per_cpu_msgs = [read_swb_per_cpu_logmsg(f'{swb_logmsg}.per_cpu_msgs[{i}]') for i in range(num_entries)]
    
    reached_end = read_bool(f'{swb_logmsg}.reached_end')
    group_balance_mask_sg = read_cpumask(f'{swb_logmsg}.group_balance_mask_sg') if reached_end else None
    group_balance_cpu_sg = read_int(f'{swb_logmsg}.group_balance_cpu_sg') if reached_end else None

    return SWBLogMsg(swb_cpus, dst_cpu, cpus, idle, dst_nr_running, dst_ttwu_pending, per_cpu_msgs, 
            group_balance_mask_sg, group_balance_cpu_sg)

class GroupType(Enum):
    group_has_spare = 0
    group_fully_busy = 1
    group_misfit_task = 2
    group_smt_balance = 3
    group_asym_packing = 4
    group_imbalanced = 5
    group_overloaded = 6

def read_group_type(typ):
    value = read_value(typ)
    for option in GroupType:
        if option.name == value:
            return option
    raise Exception(f'No matching group type for {value}')


@dataclass
class SgLbStats:
    avg_load: int
    group_load: int
    group_capacity: int
    group_util: int
    group_runnable: int
    sum_nr_running: int
    sum_h_nr_running: int
    idle_cpus: int
    group_weight: int
    group_type: GroupType
    group_asym_packing: int
    group_smt_balance: int
    group_misfit_task_load: int
    # currently we haven't compiled support
    # nr_numa_running: int
    # nr_preferred_running: int


def read_sg_lb_stats(stats) -> SgLbStats:
    avg_load = read_int(f'{stats}.avg_load')
    group_load = read_int(f'{stats}.group_load')
    group_capacity = read_int(f'{stats}.group_capacity')
    group_util = read_int(f'{stats}.group_util')
    group_runnable = read_int(f'{stats}.group_runnable')
    sum_nr_running = read_int(f'{stats}.sum_nr_running')
    sum_h_nr_running = read_int(f'{stats}.sum_h_nr_running')
    idle_cpus = read_int(f'{stats}.idle_cpus')
    group_weight = read_int(f'{stats}.group_weight')
    group_type = read_group_type(f'{stats}.group_type')
    group_asym_packing = read_int(f'{stats}.group_asym_packing')
    group_smt_balance = read_int(f'{stats}.group_smt_balance')
    group_misfit_task_load = read_int(f'{stats}.group_misfit_task_load')
    # nr_numa_running = read_int(f'{stats}.nr_numa_running')
    # nr_preferred_running = read_int(f'{stats}.nr_preferred_running')
    return SgLbStats(avg_load, group_load, group_capacity, group_util, group_runnable, sum_nr_running,
            sum_h_nr_running, idle_cpus, group_weight, group_type, group_asym_packing, group_smt_balance,
            group_misfit_task_load)


@dataclass
class FBGLogMsg:
    sd_total_load: int
    sd_total_capacity: int
    sd_avg_load: int
    sd_prefer_sibling: int
    busiest_stat: SgLbStats
    local_stat: SgLbStats
    sched_energy_enabled: Optional[bool]
    rd_perf_domain_exists: Optional[bool]
    rd_overutilized: Optional[bool]
    env_imbalance: Optional[int]

def read_fbg_logmsg(fbg_logmsg) -> FBGLogMsg:
    sd_total_load = read_int(f'{fbg_logmsg}.sd_total_load')
    sd_total_capacity = read_int(f'{fbg_logmsg}.sd_total_capacity')
    sd_avg_load = read_int(f'{fbg_logmsg}.sd_avg_load')
    sd_prefer_sibling = read_int(f'{fbg_logmsg}.sd_prefer_sibling')
    busiest_stat = read_sg_lb_stats(f'{fbg_logmsg}.busiest_stat')
    local_stat = read_sg_lb_stats(f'{fbg_logmsg}.local_stat')

    past_busiest_cores = read_bool(f'{fbg_logmsg}.past_busiest_cores')
    sched_energy_enabled = read_bool(f'{fbg_logmsg}.sched_energy_enabled') if past_busiest_cores else None

    set_rd_values = read_bool(f'{fbg_logmsg}.set_rd_values')
    rd_perf_domain_exists = read_bool(f'{fbg_logmsg}.rd_perf_domain_exists') if set_rd_values else None
    rd_overutilized = read_bool(f'{fbg_logmsg}.rd_overutilized') if set_rd_values else None

    maybe_balancing = read_bool(f'{fbg_logmsg}.maybe_balancing')
    env_imbalance = read_int(f'{fbg_logmsg}.env_imbalance') if maybe_balancing else None

    return FBGLogMsg(sd_total_load, sd_total_capacity, sd_avg_load, sd_prefer_sibling, 
            busiest_stat, local_stat, sched_energy_enabled, rd_perf_domain_exists, rd_overutilized,
            env_imbalance)


@dataclass
class FBQPerCpuLogMsg:
    cpu_id: int
    rq_type: FBQType
    rq_cfs_h_nr_running: Optional[int]
    capacity: Optional[int]
    arch_asym_cpu_priority: Optional[int]
    migration_type: Optional[MigrationType]
    cpu_load: Optional[int]
    rq_cpu_capacity: Optional[int]
    arch_scale_cpu_capacity: Optional[int]
    sd_imbalance_pct: Optional[int]
    cpu_util_cfs_boost: Optional[int]
    rq_misfit_task_load: Optional[int]

def read_fbq_per_cpu_logmsg(msg) -> FBQPerCpuLogMsg:
    cpu_id = read_int(f'{msg}.cpu_id')
    rq_type = read_fbq_type(f'{msg}.rq_type')
    past_rq_type = read_bool(f'{msg}.past_rq_type')

    rq_cfs_h_nr_running = read_int(f'{msg}.rq_cfs_h_nr_running') if past_rq_type else None
    past_nr_running = read_bool(f'{msg}.past_nr_running')

    capacity = read_int(f'{msg}.capacity') if past_nr_running else None
    past_capacity_check = read_bool(f'{msg}.past_capacity_check')

    arch_asym_cpu_priority = read_int(f'{msg}.arch_asym_cpu_priority') if past_capacity_check else None
    past_prio_check = read_bool(f'{msg}.past_prio_check')

    migration_type = read_migration_type(f'{msg}.migration_type') if past_prio_check else None

    cpu_load = read_int(f'{msg}.cpu_load') if migration_type == MigrationType.migrate_load else None
    rq_cpu_capacity = read_int(f'{msg}.rq_cpu_capacity') if migration_type == MigrationType.migrate_load else None
    arch_scale_cpu_capacity = read_int(f'{msg}.arch_scale_cpu_capacity') if migration_type == MigrationType.migrate_load else None
    sd_imbalance_pct = read_int(f'{msg}.sd_imbalance_pct') if migration_type == MigrationType.migrate_load else None

    cpu_util_cfs_boost = read_int(f'{msg}.cpu_util_cfs_boost') if migration_type == MigrationType.migrate_util else None

    rq_misfit_task_load = read_int(f'{msg}.rq_misfit_task_load') if migration_type == MigrationType.migrate_misfit else None

    return FBQPerCpuLogMsg(cpu_id, rq_type, rq_cfs_h_nr_running, capacity, arch_asym_cpu_priority, migration_type, 
            cpu_load, rq_cpu_capacity, arch_scale_cpu_capacity, sd_imbalance_pct, cpu_util_cfs_boost,
            rq_misfit_task_load)


@dataclass
class FBQLogMsg:
    capacity_dst_cpu: Optional[int]
    sched_smt_active: Optional[bool]
    arch_asym_cpu_priority_dst_cpu: Optional[int]
    per_cpu_msgs: List[FBQPerCpuLogMsg]

def read_fbq_logmsg(fbq_logmsg) -> FBQLogMsg:
    capacity_dst_cpu_set = read_bool(f'{fbq_logmsg}.capacity_dst_cpu_set')
    capacity_dst_cpu = read_int(f'{fbq_logmsg}.capacity_dst_cpu') if capacity_dst_cpu_set else None
    exec_capture_output(f'ptype {fbq_logmsg}.capacity_dst_cpu_set')
    exec_capture_output(f'ptype {fbq_logmsg}.capacity_dst_cpu')
    exec_capture_output(f'p/t {fbq_logmsg}.capacity_dst_cpu_set')
    exec_capture_output(f'p/t {fbq_logmsg}.capacity_dst_cpu')

    sched_smt_active_set = read_bool(f'{fbq_logmsg}.sched_smt_active_set')
    sched_smt_active = read_bool(f'(bool) ({fbq_logmsg}.sched_smt_active)') if sched_smt_active_set else None
    exec_capture_output(f'ptype {fbq_logmsg}.sched_smt_active_set')
    exec_capture_output(f'ptype {fbq_logmsg}.sched_smt_active')
    exec_capture_output(f'p/t {fbq_logmsg}.sched_smt_active_set')
    exec_capture_output(f'p/t {fbq_logmsg}.sched_smt_active')

    arch_asym_cpu_priority_dst_cpu_set = read_bool(f'{fbq_logmsg}.arch_asym_cpu_priority_dst_cpu_set')
    arch_asym_cpu_priority_dst_cpu = read_int(f'{fbq_logmsg}.arch_asym_cpu_priority_dst_cpu') if arch_asym_cpu_priority_dst_cpu_set else None

    num_entries = read_int(f'{fbq_logmsg}.next_per_cpu_msg_slot - {fbq_logmsg}.per_cpu_msgs')
    print('NUM ENTRIES:', num_entries)
    per_cpu_msgs = [read_fbq_per_cpu_logmsg(f'{fbq_logmsg}.per_cpu_msgs[{i}]') for i in range(num_entries)]
    return FBQLogMsg(capacity_dst_cpu, sched_smt_active, arch_asym_cpu_priority_dst_cpu, per_cpu_msgs)


@dataclass
class LBLogMsg:
    lb_env: LBEnv
    swb_logmsg: SWBLogMsg
    fbg_logmsg: FBGLogMsg
    fbq_logmsg: FBQLogMsg

def read_lb_logmsg(lb_logmsg) -> Optional[LBLogMsg]:
    runs_load_balance = read_bool(f'{lb_logmsg}.runs_load_balance')
    if runs_load_balance:
        lb_env = read_lb_env(f'{lb_logmsg}.env')
        swb_logmsg = read_swb_logmsg(f'{lb_logmsg}.swb_logmsg')
        fbg_logmsg = read_fbg_logmsg(f'{lb_logmsg}.fbg_logmsg')
        fbq_logmsg = read_fbq_logmsg(f'{lb_logmsg}.fbq_logmsg')
        return LBLogMsg(lb_env, swb_logmsg, fbg_logmsg, fbq_logmsg)
    else:
        return None
        lb_env = None


@dataclass
class RDEntryLogMsg:
    max_newidle_lb_cost: int
    continue_balancing: int
    interval: int
    need_serialize: int
    lb_logmsg: Optional[LBLogMsg]
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

def read_rebalance_domains(rd_msg, sd_count) -> RDLogMsg:
    cpu = read_int(f'{rd_msg}.cpu')
    idle = read_cpu_idle_type(f'{rd_msg}.idle')
    sched_idle_cpu = read_int(f'{rd_msg}.sched_idle_cpu')
    sd_buf = [read_rebalance_domains_entry(f'{rd_msg}.sd_buf[{i}]') for i in range(sd_count)]
    return RDLogMsg(cpu, idle, sched_idle_cpu, sd_buf)

class Encoder(json.JSONEncoder):
    def default(self, obj):
        print(obj)
        if isinstance(obj, Enum):
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

try:
    SWK
except:
    SWK = None

try:
    PORT
except:
    PORT = 1234

try:
    LOUD
except:
    LOUD = False
    
# print config to user
print('TOPOLOGY:', TOPOLOGY)
print('CORES:', CORES)
print('FILE:', FILE)
print('ITERS:', ITERS)
print('SWK:', SWK)
print('PORT:', PORT)

## Command to interact with GDB

def exec(cmd):
    if LOUD:
        print(cmd)
    gdb.execute(cmd)

def exec_capture_output(cmd):
    if LOUD:
        print(cmd)
    output = gdb.execute(cmd, False, True).strip()
    if LOUD:
        print(output)
    return output

## helper functions to extract information out 

def get_slot(rq, position=None):
    if position is None:
        position = f'{rq}->cfs.karan_logbuf.position'
    print('RQ:', read_value(rq))
    print('POSITION:', read_value(position))
    return read_value(f'{rq}->cfs.karan_logbuf.msgs[{position}]')

def get_logmsg_info(ptr, sd_count):
    if ptr == '0x0' or read_value(ptr) == '0x0':
        return None
    ptr = f'((struct karan_logmsg *) {ptr})'  # cast to the right type
    codepath = read_value(f'{ptr}->codepath')
    print('CODEPATH:', codepath)
    if codepath == 'REBALANCE_DOMAINS':
        data = read_rebalance_domains(f'{ptr}->rd_msg', sd_count)
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

def handle_single_entry():
    ptr = get_slot('rq')
    if ptr == '0x0':
        print('it\'s null...')
        exec('c')
        return
    print(ptr)
    get_logmsg_info(ptr)  # TODO: get sd_count

total_data = []
def handle_wraparound():
    global total_data
    # dump whole buffer
    buf = 'rq->cfs.karan_logbuf'
    num_entries = read_int(f'sizeof({buf}.msgs) / sizeof(*{buf}.msgs)')
    sd_count = read_int(f'{buf}.sd_count')
    print('NUM ENTRIES:', num_entries)
    print('SD COUNT:', sd_count)
    data = [get_logmsg_info(get_slot('rq', i), sd_count) for i in range(num_entries)]
    total_data.extend(data)
    if FILE is not None and SWK is None:
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
        elif typ in [2, 3]:
            handle_single_entry()
        elif typ == 4:
            handle_wraparound() # dmup
        else:
            print('oh no')

def run_swk():
    print('IN SWK HANDLER')
    print(f'SWK {SWK} ITERS {ITERS}')
    exec('en 4')
    for i in range(ITERS):
        print(f'running iter {i}')
        exec('c')
    with open(FILE, 'w') as f:
        json.dump(total_data, f, cls=Encoder)
    exec('q')
            
gdb.events.stop.connect(breakpoint_handler)

## actual script

# connect to remote

exec('set pagination off')
exec('file ~/rsch/kbuild/vmlinux')
exec(f'tar rem :{PORT}')

exec('b karan_logmsg_ready')
if SWK is None:
    while ready_count < CORES:
        exec('c')
    print('we are ready now')

# once we have made everything ready
exec('dis 1')
exec('b karan_rebalance_domains_ret')
exec('b karan_newidle_balance_ret')

exec('dis 2')
exec('dis 3')
exec('b karan_msg_alloc_wraparound')

if SWK is None:
    # go until the first example
    exec('c')
    if ITERS is not None:
        for i in range(ITERS):
            print('ON ITER', i)
            exec('c')
        json.dump(total_data, file, cls=Encoder)
        print(json.dumps(total_data, cls=Encoder))
        file.close()
else:
    exec('dis 4')
    os.system(f'kill -10 {SWK}') # send SIGUSR1 to swk
    exec('c')
    
