CPU=$1
ssh -p2222 root@localhost "mount /dev/sdb1 /mnt"
ssh -p2222 root@localhost "cat /proc/munch/$CPU > /mnt/$CPU.txt"
scp -P2222 root@localhost:"/mnt/$CPU.txt" "$CPU.txt"
