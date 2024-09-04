import gdb

def exec_capture_output(cmd):
    return gdb.execute(cmd, False, True)

def get_logmsg_ptr(output):
    # looks like: "$1 = (struct karan_logmsg *) 0x0 <fixed_percpu_data>"
    return output.split()[5]

def get_entry(rq):
    position = f'{rq}->cfs.karan_logbuf.position'
    msg_size = f'{rq}->cfs.karan_logbuf.msg_size'
    output = exec_capture_output(f'p {rq}->cfs.karan_logbuf.msgs[{position} * {msg_size}]')
    return get_logmsg_ptr(output)

def handle_rebalance_domains():
    print('handling rebalance domains...')
    ptr = get_entry('rq')
    
    if ptr == '0x0':
        print('it\'s null...')
        gdb.execute('c')
        return
    print(ptr)

def handle_newidle_balance():
    print('handling newidle balance...')
    ptr = get_entry('this_rq')
    
    if ptr == '0x0':
        print('it\'s null...')
        gdb.execute('c')
        return
    print(ptr)

def handle_karan_newidle_balance_ret():
    print('handling karan_newidle_balance_ret...')
    ptr = get_logmsg_ptr(exec_capture_output('p buf'))
    
    if ptr == '0x0':
        print('it\'s null...')
        gdb.execute('c')
        return
    print(ptr)

def breakpoint_handler(event):
    if isinstance(event, gdb.BreakpointEvent):
        bp = event.breakpoints[0]
        typ = bp.number
        if typ == 1:
            handle_rebalance_domains()
        elif typ == 2:
            handle_newidle_balance()
        else:
            handle_karan_newidle_balance_ret()

gdb.events.stop.connect(breakpoint_handler)

# connect to remote
gdb.execute('file /l/kbuild/vmlinux')
gdb.execute('tar rem :1234')

gdb.execute('b rebalance_domains:out')
gdb.execute('b newidle_balance:out')
gdb.execute('b karan_newidle_balance_ret')

gdb.execute('c')
