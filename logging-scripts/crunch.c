#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>

#define MATSIZE (dim * dim * sizeof(double))

int iters = 256;
int dim = 256;

void fill (void *buf, int fd, int n) {
    int pagesize = getpagesize();
    void *end = buf + n;
    for (; buf < end; buf += read(fd, buf, (end - buf < pagesize) ? (end - buf) : pagesize));
}

struct thread_args {
    int id;
    double *a;
    double *b;
    double *c;
};

void multiply (double *a, double *b, double *c) {
    for (int i = 0; i < dim; i++) {
        for (int j = 0; j < dim; j++) {
            double accum = 0;
            for (int k = 0; k < dim; k++) {
                accum += a[i * dim + k] * b[k * dim + j];
            }
            c[i * dim + j] = accum;
        }
    }
}

void *thread_func (void *args_) {
    struct thread_args *args = (struct thread_args *) args_;
    printf("thread %d start\n", args->id);
    for (int i = 0; i < iters; i++) {
        multiply(args->a, args->b, args->c);
        memcpy(args->a, args->c, MATSIZE);
    }
    printf("thread %d end\n", args->id);
    return NULL;
}

int main (int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s nthreads\n", argv[0]);
        return 1;
    }

    int nthreads = strtol(argv[1], NULL, 10);
    int tokill = 0; //argc == 3 ? strtol(argv[2], NULL, 10) : 0; 
    char *eptr = NULL;
    if (eptr = getenv("ITERS")) {
        iters = strtol(eptr, NULL, 10);
    }
    if (eptr = getenv("DIM")) {
        dim = strtol(eptr, NULL, 10);
    }
    if (eptr = getenv("TOKILL")) {
        tokill = strtol(eptr, NULL, 10);
    }
    printf("nthreads %d iters %d dim %d tokill %d\n", nthreads, iters, dim, tokill);

    struct thread_args targs[nthreads];
    int fd = open("/dev/urandom", O_RDONLY);    
    for (int i = 0; i < nthreads; i++) {
        targs[i].id = i;

        void *dat = (double *) malloc(3 * MATSIZE);
        fill(dat, fd, 3 * MATSIZE);

        targs[i].a = (double *) dat;
        targs[i].b = (double *) (dat + MATSIZE);
        targs[i].c = (double *) (dat + 2 * MATSIZE);
    }
    close(fd);

    printf("spawning threads\n");
    pthread_t threads[nthreads];
    for (int i = 0; i < nthreads; i++) {
        pthread_create(&threads[i], NULL, thread_func, (void *) &targs[i]);
    }

    printf("all threads spawned");
    if (tokill) {
        printf(", killing tokill\n");
        kill(tokill, 9);
    } else {
        printf("\n");
    }
    
    for (int i = 0; i < nthreads; i++) {
        pthread_join(threads[i], NULL);
    }
    
    return 0;
}
