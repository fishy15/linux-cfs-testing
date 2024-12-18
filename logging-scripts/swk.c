#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/types.h>
#include <unistd.h>

#define CHECK_OK(a) a == 0 ? 0 : (fprintf(stderr, "%s failed w errno %d at %s:%d\n", #a, errno, __FILE__, __LINE__), exit(1));

#define CHECK_EQ(a, b) a == b ? 0 : (fprintf(stderr, "%s != %s at %s:%d\n", #a, #b, __FILE__, __LINE__), exit(1));

#define CHECK(a) a ? 0 : (fprintf(stderr, "condition %s failed at %s:%d\n", #a, __FILE__, __LINE__), exit(1));

int p0[2];
int p1[2];
pid_t gdb = 0;

void cleanup (int sig) {
    printf("signal %d received\n", sig);
    //if (grep) (printf("killing grep [pid %d]\n", grep), kill(grep, 9));
    if (gdb) (printf("killing gdb [pid %d]\n", gdb), kill(gdb, 9));
    printf("exiting--bye!\n");
    exit(0);
}

void start_workload();
void collect_data();
void normal_out();

void handle_sig (int sig) {
    if (sig == 10) { // SIGUSR1, probably
        printf("lets do this\n");
        cleanup(10);
    } else {
        cleanup(sig);
    }
}

void run_gdb (char *topo, char *outfile, pid_t ppid) {
    char topoarg[1024];
    bzero(topoarg, 1024);
    snprintf(topoarg, 1024, "py TOPOLOGY=\"%s\"", topo);
    
    char filearg[1024];
    bzero(filearg, 1024);
    snprintf(filearg, 1024, "py FILE=\"%s\"", outfile);

    char swkarg[1024];
    bzero(swkarg, 1024);
    snprintf(swkarg, 1024, "py SWK=\"%d\"", ppid);
    
    execl("/usr/bin/gdb",
          "gdb",
          "/local/research/kbuild/vmlinux",
          "-ex",
          topoarg,
          "-ex",
          filearg,
          "-ex",
          swkarg,
          "-x",
          "/local/research/kernel/dumper.py",
          NULL);
}

int main (int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s outfile\n", argv[0]);
        return 1;
    }
    char *topo = getenv("TOPOLOGY");
    if (topo == NULL) topo = "2";

    // register cleanup handler
    struct sigaction sact;
    sact.sa_handler = handle_sig;
    sigaction(2, &sact, NULL);
    sigaction(10, &sact, NULL);
    
    CHECK_OK(pipe2(p0, 0));
    CHECK_OK(pipe2(p1, 0));
    
    // setup gdb
    gdb = fork();
    CHECK(gdb >= 0);
    if (gdb == 0) {
        CHECK_EQ(dup2(p0[0], 0), 0);
        //CHECK_EQ(dup2(p1[1], 1), 1);
        run_gdb(topo, argv[1], getppid());
    }
    CHECK(gdb > 0);

    pause();
}
