#ifndef _LINUX_MUNCH_H
#define _LINUX_MUNCH_H

// location enum
enum munch_location {
	MUNCH_CPU_NUMBER,
};

struct munch_ops {
	void (*munch64) (size_t, enum munch_location, uint64_t);
	size_t (*open_meal)(void);
};

void munch64(size_t, enum munch_location, uint64_t);
size_t open_meal(void);

void set_muncher (struct munch_ops *);

#endif
