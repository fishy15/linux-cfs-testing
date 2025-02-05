#ifndef _LINUX_MUNCH_H
#define _LINUX_MUNCH_H

struct munch_ops {
    void (*munch64) (uint64_t);
};

void munch64 (uint64_t);
void set_muncher (struct munch_ops *);

#endif
