#ifndef _LINUX_MUNCH_H
#define _LINUX_MUNCH_H

#include "sched/idle.h"
#include <linux/sched/idle.h>

// flag enum
enum munch_flag {
	MUNCH_GO_TO_NEXT_SD,
};

enum munch_location_bool {
	MUNCH_SWB_RESULT,
	MUNCH_ASYM_CPUCAPACITY,
	MUNCH_HAS_BUSIEST,
};

enum munch_location_u64 {
	MUNCH_CPU_NUMBER,
	MUNCH_SD_AVG_LOAD,
	MUNCH_IMBALANCE_PCT,
};

enum munch_location_u64_cpu {
	MUNCH_NR_RUNNING,
	MUNCH_CPU_CAPACITY,
	MUNCH_ASYM_CPU_PRIORITY_VALUE,
};

enum munch_location_bool_cpu {
	MUNCH_IDLE_CPU,
	MUNCH_IS_CORE_IDLE,
	MUNCH_TTWU_PENDING,
	MUNCH_RD_OVERUTILIZED,
	MUNCH_RD_PD_OVERLAP,
};

enum munch_location_u64_group {
	MUNCH_SUM_H_NR_RUNNING,
	MUNCH_SUM_NR_RUNNING,
	MUNCH_SGC_MAX_CAPACITY,
	MUNCH_SGC_MIN_CAPACITY,
	MUNCH_SG_AVG_LOAD,
	MUNCH_SG_ASYM_PREFER_CPU,
	MUNCH_MISFIT_TASK_LOAD,
	MUNCH_SG_IDLE_CPUS,
	MUNCH_GROUP_BALANCE_CPU,
};

struct meal_descriptor {
	size_t age;
	size_t cpu_number;
	size_t entry_idx;
};

struct munch_iterator {
	size_t cpu;
	size_t entry_index;
	size_t sd_index;
	bool sd_main_finished;
	size_t sg_index;
	size_t cpu_index;
};

struct munch_ops {
	void (*munch_flag) (struct meal_descriptor *, enum munch_flag);
	void (*munch_bool) (struct meal_descriptor *, enum munch_location_bool, bool);
	void (*munch64) (struct meal_descriptor *, enum munch_location_u64, uint64_t);
	void (*munch_cpu_idle_type) (struct meal_descriptor *, enum cpu_idle_type);
	void (*munch_bool_cpu) (struct meal_descriptor *, enum munch_location_bool_cpu, size_t, bool);
	void (*munch_u64_cpu) (struct meal_descriptor *, enum munch_location_u64_cpu, size_t, uint64_t);
	void (*munch_u64_group) (struct meal_descriptor *, enum munch_location_u64_group, const struct sched_group *sg, uint64_t);
	void (*open_meal) (size_t, struct meal_descriptor *);
	void (*close_meal) (struct meal_descriptor *);

	// dump sequence
	void (*start_dump) (size_t);
	ssize_t (*dump_data) (struct seq_file *, const struct munch_iterator *);
	void (*move_iterator) (struct munch_iterator *);
	void (*finalize_dump) (size_t);
};

void munch_flag(struct meal_descriptor *, enum munch_flag);
bool munch_bool(struct meal_descriptor *, enum munch_location_bool, bool);
uint64_t munch_u64(struct meal_descriptor *, enum munch_location_u64, uint64_t);
enum cpu_idle_type munch_cpu_idle_type(struct meal_descriptor *, enum cpu_idle_type);
bool munch_bool_cpu(struct meal_descriptor *, enum munch_location_bool_cpu, size_t, bool);
uint64_t munch_u64_cpu(struct meal_descriptor *, enum munch_location_u64_cpu, size_t, uint64_t);
uint64_t munch_u64_group(struct meal_descriptor *, enum munch_location_u64_group, const struct sched_group *sg, uint64_t);
void open_meal(size_t, struct meal_descriptor *);
void close_meal(struct meal_descriptor *);

void set_muncher (struct munch_ops *);

// procfs
int munch_register_procfs(void);
void munch_unregister_procfs(void);
bool munch_seq_has_overflowed(struct seq_file *m);

// get info from kernel
size_t nr_sched_domains(size_t cpu);
const struct sched_domain *get_sd(size_t cpu, size_t sd_index);
size_t nr_sched_groups(const struct sched_domain *sd);
const struct sched_group *get_sg(const struct sched_domain *sd, size_t sg_index);

extern const size_t MUNCH_NUM_ENTRIES;

#endif
