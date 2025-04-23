CPU=$1
ssh "-p$SSH" k@localhost "cat /proc/munch/$CPU" > "$CPU.txt"

echo "trying to kill $2"
kill -10 $2
