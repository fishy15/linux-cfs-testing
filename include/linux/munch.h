#ifndef _LINUX_MUNCH_H
#define _LINUX_MUNCH_H

#include "sched/idle.h"
#include <linux/sched/idle.h>

// flag enum
enum munch_flag {
	MUNCH_GO_TO_NEXT_SD,
};

enum munch_location_bool {
	MUNCH_BOOL_TODO, // placeholder
};

enum munch_location_u64 {
	MUNCH_CPU_NUMBER,
	MUNCH_GROUP_BALANCE_CPU_SG,
};

enum munch_location_u64_cpu {
	MUNCH_NR_RUNNING,
};

enum munch_location_bool_cpu {
	MUNCH_IDLE_CPU,
	MUNCH_IS_CORE_IDLE,
	MUNCH_TTWU_PENDING,
};

struct meal_descriptor {
	size_t age;
	size_t cpu_number;
	size_t entry_idx;
};

struct munch_ops {
	void (*munch_flag) (struct meal_descriptor *, enum munch_flag);
	void (*munch_bool) (struct meal_descriptor *, enum munch_location_bool, bool);
	void (*munch64) (struct meal_descriptor *, enum munch_location_u64, uint64_t);
	void (*munch_cpu_idle_type) (struct meal_descriptor *, enum cpu_idle_type);
	void (*munch_bool_cpu) (struct meal_descriptor *, enum munch_location_bool_cpu, size_t, bool);
	void (*munch_u64_cpu) (struct meal_descriptor *, enum munch_location_u64_cpu, size_t, u64);
	void (*open_meal) (size_t, struct meal_descriptor *);
	void (*close_meal) (struct meal_descriptor *);

	// dump sequence
	void (*start_dump) (size_t cpu);
	ssize_t (*dump_data) (struct seq_file *m, size_t cpu, size_t entry_index);
	void (*finalize_dump) (size_t cpu);
};

void munch_flag(struct meal_descriptor *, enum munch_flag);
bool munch_bool(struct meal_descriptor *, enum munch_location_bool, bool);
uint64_t munch_u64(struct meal_descriptor *, enum munch_location_u64, uint64_t);
enum cpu_idle_type munch_cpu_idle_type(struct meal_descriptor *md, enum cpu_idle_type);
bool munch_bool_cpu(struct meal_descriptor *, enum munch_location_bool_cpu, size_t, bool);
uint64_t munch_u64_cpu(struct meal_descriptor *, enum munch_location_u64_cpu, size_t, uint64_t);
void open_meal(size_t, struct meal_descriptor *);
void close_meal(struct meal_descriptor *);

void set_muncher (struct munch_ops *);

// procfs
int munch_register_procfs(void);
void munch_unregister_procfs(void);
bool munch_seq_has_overflowed(struct seq_file *m);

// get info from kernel
size_t nr_sched_domains(size_t cpu);

extern const size_t MUNCH_NUM_ENTRIES;

#endif
