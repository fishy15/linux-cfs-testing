#include <stdio.h>
#include <sys/types.h>
#include <unistd.h>

void main () {
    printf("%d\n", getpid());
    pause();
}
