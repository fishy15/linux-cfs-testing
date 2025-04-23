#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#define CHECK_OK(a) a == 0 ? 0 : (fprintf(stderr, "%s failed at %s:%d, errno %d\n", #a, __FILE__, __LINE__, errno), cleanup(1));

#define CHECK_EQ(a, b) a == b ? 0 : (fprintf(stderr, "%s != %s at %s:%d, errno %d\n", #a, #b, __FILE__, __LINE__, errno), cleanup(1));

#define CHECK_NE(a, b) a != b ? 0 : (fprintf(stderr, "%s == %s at %s:%d, errno %d\n", #a, #b, __FILE__, __LINE__, errno), cleanup(1));

#define CHECK(a) a ? 0 : (fprintf(stderr, "condition %s failed at %s:%d, errno %d\n", #a, __FILE__, __LINE__, errno), cleanup(1));

int pipe_gdb[2];
int pipe_waitfor[2];
pid_t gdb = 0;
pid_t ssh_cmd = 0;
pid_t ssh_waitfor = 0;
char *topo;
char *outfile;
char *cmd;
int gdb_port = 1234;
int ssh_port = 2222;
int iters = 1;

void ready_ssh () {
    char ssh_invoc[1024];
    bzero(ssh_invoc, 1024);
    snprintf(ssh_invoc, 1024, "ssh -p%d -o 'StrictHostKeyChecking=no' k@localhost whoami", ssh_port);

    while (system(ssh_invoc) != 0);
}

void run_waitfor () {
    ssh_waitfor = fork();
    CHECK(ssh_waitfor >= 0);
    if (ssh_waitfor == 0) {
        CHECK_OK(close(0));
        CHECK(posix_openpt(O_RDWR | O_NOCTTY) == 0);

        CHECK_EQ(dup2(pipe_waitfor[1], 1), 1);

        char portarg[1024];
        bzero(portarg, 1024);
        snprintf(portarg, 1024, "-p%d", ssh_port);

        execl("/usr/bin/ssh",
              "ssh",
              portarg,
	      "-o",
	      "StrictHostKeyChecking=no",
              "-t",
              "k@localhost",
              "stdbuf -o0 waitfor",
              NULL);
    }
}

int get_tokill () {
    char _tokill[8];
    bzero(_tokill, 8);

    char *i = _tokill;
    while (1) {
        CHECK_EQ(read(pipe_waitfor[0], i, 1), 1);
        if (*i == '\n') {
            int tokill = strtol(_tokill, NULL, 10);
            printf("retrieved tokill %d\n", tokill);
            return tokill;
        }
        i++;
    }

    CHECK(0);
}

void run_cmd (int tokill) {
    ssh_cmd = fork();
    CHECK(ssh_cmd >= 0);
    if (ssh_cmd == 0) {
        CHECK_OK(close(0));
        CHECK(posix_openpt(O_RDWR | O_NOCTTY) == 0);
        
        char portarg[1024];
        bzero(portarg, 1024);
        snprintf(portarg, 1024, "-p%d", ssh_port);
        
        char cmdarg[1024];
        bzero(cmdarg, 1024);
        snprintf(cmdarg, 1024, "stdbuf -o0 sh -c 'TOKILL=%d %s'", tokill, cmd);
        printf("cmdarg is \"%s\"\n", cmdarg);
        
        execl("/usr/bin/ssh",
              "ssh",
              "-o",
              "StrictHostKeyChecking=no",
              portarg,
              "-t", // need to pseudo tty to make sure SIGINTs go through
              "k@localhost",
              cmdarg,
              NULL);
    }
    CHECK(ssh_cmd > 0);
}

void cleanup (int code) {
    if (gdb) (printf("killing gdb [pid %d]\n", gdb), kill(gdb, 5)); // help
    if (ssh_cmd) (printf("killing ssh_cmd [pid %d]\n", ssh_cmd), kill(ssh_cmd, 2));
    if (ssh_waitfor) (printf("killing ssh_waitfor [pid %d]\n", ssh_waitfor), kill(ssh_waitfor, 2));
    printf("exiting--bye!\n");
    exit(code);
}

void handle_sig (int sig) {
    if (sig == 10) { // SIGUSR1, probably
        printf("====lets do this====\n");

        run_waitfor(); // spawn waitfor
        int tokill = get_tokill();
        run_cmd(tokill); // cmd will kill waitfor when ready to be profiled
        waitpid(ssh_waitfor, NULL, 0);
        
        kill(gdb, 2); // interrupt into gdb
        CHECK_EQ(write(pipe_gdb[1], "py run_swk()\n", 13), 13);
        
        waitpid(gdb, NULL, 0);
        kill(ssh_cmd, 2); // end cmd when profiling is finished
        printf("====this is done, returning====\n");
    } else {
        printf("signal %d received\n", sig);
        cleanup(0);
    }
}

void run_gdb (pid_t ppid) {
    gdb = fork();
    CHECK(gdb >= 0);
    if (gdb == 0) {
        CHECK_EQ(dup2(pipe_gdb[0], 0), 0);
        
        execl("copy-file.sh",
              "0.txt",
              NULL);
    }
    CHECK(gdb > 0);
}

int main (int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s outfile cmd\n", argv[0]);
        return 1;
    }

    topo = getenv("TOPOLOGY");
    if (topo == NULL) topo = "2";

    char cwd[PATH_MAX];
    CHECK_NE(getcwd(cwd, PATH_MAX), NULL);
    outfile = (char *) malloc(PATH_MAX);
    snprintf(outfile, PATH_MAX, "%s/%s", cwd, argv[1]);
    
    cmd = argv[2];

    char *_ssh_port = getenv("SSH");
    if (_ssh_port) ssh_port = strtol(_ssh_port, NULL, 10);

    printf("ssh_port is %d\n", ssh_port);
    printf("outfile is %s\n", outfile);
    printf("cmd is %s\n", cmd);

    ready_ssh();
    
    // register cleanup handler
    struct sigaction sact;
    sact.sa_handler = handle_sig;
    sigaction(2, &sact, NULL);
    sigaction(10, &sact, NULL);
    
    CHECK_OK(pipe2(pipe_gdb, 0));
    CHECK_OK(pipe2(pipe_waitfor, 0));

    run_gdb(getpid());

    pause();

    printf("====unpause====\n");
    printf("done!\n");
}
