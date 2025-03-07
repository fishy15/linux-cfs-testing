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
	void (*munch64) (struct meal_descriptor *, enum munch_location, uint64_t);
	void (*open_meal) (size_t, struct meal_descriptor *);
	void (*close_meal) (struct meal_descriptor *);
	ssize_t (*dump_data) (char *buf, size_t length, size_t cpu);
};

void munch64(struct meal_descriptor *, enum munch_location, uint64_t);
void open_meal(size_t, struct meal_descriptor *);
void close_meal(struct meal_descriptor *);

void set_muncher (struct munch_ops *);

// procfs
int munch_register_procfs(void);
void munch_unregister_procfs(void);

// get info from kernel
size_t nr_sched_domains(size_t cpu);

#endif
