#ifndef _LINUX_MUNCH_H
#define _LINUX_MUNCH_H

struct munch_ops {
	void (*munch64) (uint64_t);
};

void munch64 (uint64_t);
void set_muncher (struct munch_ops *);

// location enum
enum munch_location {
	MUNCH_CPU_NUMBER,
};

#endif
