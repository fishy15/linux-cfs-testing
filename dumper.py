import gdb

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

def exec(cmd):
    print(cmd)
    gdb.execute(cmd)

def exec_capture_output(cmd):
    print(cmd)
    return gdb.execute(cmd, False, True)

def get_logmsg_ptr(output):
    # looks like: "$1 = (struct karan_logmsg *) 0x0 <fixed_percpu_data>"
    print('OUTPUT:', output)
    return output.split()[5]

def get_slot(rq):
    position = f'{rq}->cfs.karan_logbuf.position'
    print('RQ:', exec_capture_output(f'p {rq}'), end='')
    print('POSITION:', exec_capture_output(f'p {position}'), end='')
    output = exec_capture_output(f'p {rq}->cfs.karan_logbuf.msgs[{position}]')
    return get_logmsg_ptr(output)

ready_count = 0
def handle_ready():
    global ready_count
    ready_count += 1

def handle_rebalance_domains():
    print('handling rebalance domains...')
    ptr = get_slot('rq')
    
    if ptr == '0x0':
        print('it\'s null...')
        exec('c')
        return
    print(ptr)

def handle_newidle_balance():
    print('handling newidle balance...')
    ptr = get_slot('this_rq')
    
    if ptr == '0x0':
        print('it\'s null...')
        exec('c')
        return
    print(ptr)

def handle_karan_newidle_balance_ret():
    # ignore for now
    return

    print('handling karan_newidle_balance_ret...')
    ptr = get_logmsg_ptr(exec_capture_output('p buf'))
    
    if ptr == '0x0':
        print('it\'s null...')
        exec('c')
        return
    print(ptr)

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

print(exec_capture_output('show non-stop'))

while True:
    exec('c')
