import gdb

# connect to remote
gdb.execute('file /l/kbuild/vmlinux')
gdb.execute('tar rem :1234')

gdb.execute('b rebalance_domains:out')
gdb.execute('b newidle_balance:out')
gdb.execute('b karan_newidle_balance_ret')

gdb.execute('c')
