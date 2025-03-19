#ifndef _LINUX_MUNCH_H
#define _LINUX_MUNCH_H

#include "sched/idle.h"
#include <linux/sched/idle.h>

// flag enum
enum munch_flag {
	MUNCH_GO_TO_NEXT_SD,
};

enum munch_location_bool {
	MUNCH_DST_RQ_TTWU_PENDING,
};


enum munch_location_u64 {
	MUNCH_CPU_NUMBER,
	MUNCH_DST_RQ_NR_RUNNING,
};

struct meal_descriptor {
	size_t cpu_number;
	size_t entry_idx;
};

struct munch_ops {
	void (*munch_flag) (struct meal_descriptor *, enum munch_flag);
	void (*munch_bool) (struct meal_descriptor *, enum munch_location_bool, bool);
	void (*munch64) (struct meal_descriptor *, enum munch_location_u64, uint64_t);
	void (*munch_cpu_idle_type) (struct meal_descriptor *, enum cpu_idle_type);
	void (*open_meal) (size_t, struct meal_descriptor *);
	void (*close_meal) (struct meal_descriptor *);
	ssize_t (*dump_data) (char *buf, size_t length, size_t cpu);
	void (*finalize_dump) (size_t cpu);
};

void munch_flag(struct meal_descriptor *, enum munch_flag);
bool munch_bool(struct meal_descriptor *, enum munch_location_bool, bool);
uint64_t munch_u64(struct meal_descriptor *, enum munch_location_u64, uint64_t);
enum cpu_idle_type munch_cpu_idle_type(struct meal_descriptor *md, enum cpu_idle_type);
void open_meal(size_t, struct meal_descriptor *);
void close_meal(struct meal_descriptor *);

void set_muncher (struct munch_ops *);

// procfs
int munch_register_procfs(void);
void munch_unregister_procfs(void);

// get info from kernel
size_t nr_sched_domains(size_t cpu);

#endif
