CPU=$1
ssh -p2222 root@localhost "cat /proc/munch/$CPU > ~/$CPU.txt"
scp -P2222 root@localhost:"~/$CPU.txt" "$CPU.txt"
