#ifndef _LINUX_MUNCH_H
#define _LINUX_MUNCH_H

// location enum
enum munch_location {
	MUNCH_CPU_NUMBER,
};

struct meal_descriptor {
	size_t cpu_number;
	size_t entry_idx;
};

struct munch_ops {
	void (*munch64) (size_t, enum munch_location, uint64_t);
	void (*open_meal) (size_t, struct meal_descriptor *);
};

void munch64(size_t, enum munch_location, uint64_t);
void open_meal(size_t, struct meal_descriptor *);

void set_muncher (struct munch_ops *);

#endif
