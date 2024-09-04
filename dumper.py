import gdb

def handle_rebalance_domains():
    print('handling rebalance domains...')

def handle_newidle_balance():
    print('handling newidle balance...')

def handle_karan_newidle_balance_ret():
    print('handling karan_newidle_balance_ret...')

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


