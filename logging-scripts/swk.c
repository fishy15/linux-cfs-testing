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
#include <unistd.h>

#include <libssh/libssh.h>

#define CHECK_OK(a) a == 0 ? 0 : (fprintf(stderr, "%s failed at %s:%d, errno %d\n", #a, __FILE__, __LINE__, errno), cleanup(1));

#define CHECK_EQ(a, b) a == b ? 0 : (fprintf(stderr, "%s != %s at %s:%d, errno %d\n", #a, #b, __FILE__, __LINE__, errno), cleanup(1));

#define CHECK_NE(a, b) a != b ? 0 : (fprintf(stderr, "%s == %s at %s:%d, errno %d\n", #a, #b, __FILE__, __LINE__, errno), cleanup(1));

#define CHECK(a) a ? 0 : (fprintf(stderr, "condition %s failed at %s:%d, errno %d\n", #a, __FILE__, __LINE__, errno), cleanup(1));

int p0[2];
pid_t gdb = 0;
pid_t ssh = 0;
char *topo;
char *outfile;
char *cmd;
ssh_session sesh;
int gdb_port = 1234;
int ssh_port = 2222;

void run_ssh () {
    char portarg[1024];
    bzero(portarg, 1024);
    snprintf(portarg, 1024, "-p%d", ssh_port);
    
    ssh = fork();
    CHECK(ssh >= 0);
    if (ssh == 0) {
        execl("/usr/bin/ssh",
              "ssh",
              portarg,
              "k@localhost",
              cmd,
              NULL);
    }
    CHECK(ssh > 0);
}

void cleanup (int code) {
    //if (grep) (printf("killing grep [pid %d]\n", grep), kill(grep, 9));
    if (gdb) (printf("killing gdb [pid %d]\n", gdb), kill(gdb, 9));
    if (ssh) (printf("killing ssh [pid %d]\n", ssh), kill(ssh, 9));
    printf("exiting--bye!\n");
    exit(code);
}

void handle_sig (int sig) {
    if (sig == 10) { // SIGUSR1, probably
        printf("====lets do this====\n");

        run_ssh();
        sleep(1); // let the workload get ready
        kill(gdb, 2); // gdb int
        CHECK_EQ(write(p0[1], "en 5\n", 5), 5);
        CHECK_EQ(write(p0[1], "c\n", 2), 2);

        printf("====this is done, returning====\n");
    } else {
        printf("signal %d received\n", sig);
        cleanup(0);
    }
}

void run_gdb (pid_t ppid) {
    char topoarg[1024];
    bzero(topoarg, 1024);
    snprintf(topoarg, 1024, "py TOPOLOGY=\"%s\"", topo);
    
    char filearg[1024];
    bzero(filearg, 1024);
    snprintf(filearg, 1024, "py FILE=\"%s\"", outfile);

    char swkarg[1024];
    bzero(swkarg, 1024);
    snprintf(swkarg, 1024, "py SWK=\"%d\"", ppid);

    char portarg[1024];
    bzero(portarg, 1024);
    snprintf(portarg, 1024, "py PORT=\"%d\"", gdb_port);

    char rslv[PATH_MAX];
    char to_rslv[PATH_MAX];
    char *home = getenv("HOME");
    CHECK_NE(home, NULL);
    snprintf(to_rslv, PATH_MAX, "%s/rsch/kbuild/", home);
    CHECK_NE(realpath(to_rslv, rslv), NULL);
    CHECK_OK(chdir(rslv));
    
    execl("/usr/bin/gdb",
          "gdb",
          "vmlinux",
          "-ex",
          topoarg,
          "-ex",
          filearg,
          "-ex",
          swkarg,
          "-ex",
          portarg,
          "-x",
          "../kernel/dumper.py",
          NULL);
}

/*
void start_workload () {
    sesh = ssh_new();
    CHECK_NE(sesh, NULL);
    
    ssh_options_set(my_ssh_session, SSH_OPTIONS_HOST, "localhost");
    ssh_options_set(my_ssh_session, SSH_OPTIONS_PORT, 2222);
    ssh_options_set(my_ssh_session, SSH_OPTIONS_USER, "k");
    
    CHECK_EQ(ssh_connect(sesh), SSH_OK);
    CHECK_EQ(ssh_userauth_password(sesh, NULL, "k"), SSH_AUTH_SUCCESS);

    ssh_channel ch = ssh_channel_new(session);
    CHECK_NE(ch, NULL);
    
    
    ssh_disconnect(sesh);
    ssh_free(sesh);
}
*/

int main (int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s outfile cmd\n", argv[0]);
        return 1;
    }
    topo = getenv("TOPOLOGY");
    if (topo == NULL) topo = "2";
    outfile = argv[1];
    cmd = argv[2];

    char *_gdb_port = getenv("GDB");
    if (_gdb_port) gdb_port = strtol(_gdb_port, NULL, 10);
    char *_ssh_port = getenv("SSH");
    if (_ssh_port) ssh_port = strtol(_ssh_port, NULL, 10);

    printf("gdb_port is %d\n", gdb_port);
    printf("ssh_port is %d\n", ssh_port);
    printf("cmd is %s\n", cmd);
    
    // register cleanup handler
    struct sigaction sact;
    sact.sa_handler = handle_sig;
    sigaction(2, &sact, NULL);
    sigaction(10, &sact, NULL);
    
    CHECK_OK(pipe2(p0, 0));
    //CHECK_OK(pipe2(p1, 0));
    
    // setup gdb
    gdb = fork();
    CHECK(gdb >= 0);
    if (gdb == 0) {
        CHECK_EQ(dup2(p0[0], 0), 0);
        //CHECK_EQ(dup2(p1[1], 1), 1);
        run_gdb(getppid());
    }
    CHECK(gdb > 0);

    pause();

    printf("====unpause====\n");
    pause();
}
