#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <strings.h>
#include <sys/types.h>
#include <unistd.h>

pid_t make = 0;

void handler (int sig) {
    printf("received signal %d\n", sig);
    if (make) kill(make, 2);
    exit(0);
}

int main (int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: TOKILL=pid %s njobs\n", argv[0]);
        exit(1);
    }

    char *_tokill = getenv("TOKILL");
    if (_tokill == NULL) {
        fprintf(stderr, "usage: TOKILL=pid %s njobs\n", argv[0]);
        exit(1);
    }
    
    int tokill = strtol(_tokill, NULL, 10);
    int njobs = strtol(argv[1], NULL, 10);

    chdir("/home/k/kbuild");
    system("make clean");
    
    struct sigaction sa;
    sa.sa_handler = handler;
    sigaction(2, &sa, NULL);
    
    make = fork();
    if (make < 0) {
        printf("fork failed %d\n", errno);
    } else if (make == 0) {
        char jobsarg[1024];
        bzero(jobsarg, 1024);
        snprintf(jobsarg, 1024, "-j%d", njobs);
        
        execl("/usr/bin/make",
              "make",
              jobsarg,
              NULL);
    }
    
    sleep(6);
    kill(tokill, 9);
    pause();
}
