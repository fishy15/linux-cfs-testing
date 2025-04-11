#include <linux/munch.h>
#include <linux/proc_fs.h>

struct munch_ops muncher;
bool is_muncher_valid = false;

const size_t MUNCH_NUM_ENTRIES = 256;

void munch_flag(struct meal_descriptor *md, enum munch_flag flag) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_flag(md, flag);
	}
}

bool munch_bool(struct meal_descriptor *md, enum munch_location_bool location, bool x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_bool(md, location, x);
	}
	return x;
}


uint64_t munch_u64(struct meal_descriptor *md, enum munch_location_u64 location, uint64_t x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch64(md, location, x);
	}
	return x;
}

const struct cpumask *munch_cpumask(struct meal_descriptor *md, const struct cpumask *x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_cpumask(md, x);
	}
	return x;
}

enum fbq_type munch_fbq_type(struct meal_descriptor *md, enum fbq_type x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_fbq_type(md, x);
	}
	return x;
}

enum migration_type munch_migration_type(struct meal_descriptor *md, enum migration_type x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_migration_type(md, x);
	}
	return x;
}

bool munch_bool_cpu(struct meal_descriptor *md, enum munch_location_bool_cpu location, size_t cpu, bool x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_bool_cpu(md, location, cpu, x);
	}
	return x;
}

uint64_t munch_u64_cpu(struct meal_descriptor *md, enum munch_location_u64_cpu location, size_t cpu, uint64_t x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_u64_cpu(md, location, cpu, x);
	}
	return x;
}

enum cpu_idle_type munch_cpu_idle_type_cpu(struct meal_descriptor *md, size_t cpu, enum cpu_idle_type x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_cpu_idle_type_cpu(md, cpu, x);
	}
	return x;
}

enum fbq_type munch_fbq_type_cpu(struct meal_descriptor * md, size_t cpu, enum fbq_type x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_fbq_type_cpu(md, cpu, x);
	}
	return x;
}

uint64_t munch_u64_group(struct meal_descriptor *md, enum munch_location_u64_group location, const struct sched_group *sg, uint64_t x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_u64_group(md, location, sg, x);
	}
	return x;
}

const struct cpumask *munch_cpumask_group(struct meal_descriptor *md, const struct sched_group *sg, const struct cpumask *x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_cpumask_group(md, sg, x);
	}
	return x;
}

enum group_type munch_group_type_group(struct meal_descriptor *md, const struct sched_group *sg, enum group_type x) {
	if (is_muncher_valid && md != NULL) {
		muncher.munch_group_type_group(md, sg, x);
	}
	return x;
}

void set_muncher(struct munch_ops *m) {
	memcpy(&muncher, m, sizeof(struct munch_ops));
	is_muncher_valid = true;
}

void open_meal(size_t cpu_number, struct meal_descriptor *md) {
	if (is_muncher_valid && md != NULL) {
		return muncher.open_meal(cpu_number, md);
	}
}

void close_meal(struct meal_descriptor *md) {
	if (is_muncher_valid && md != NULL) {
		return muncher.close_meal(md);
	}
}

// procfs

#define PROCFS_NAME "munch" 
#define GET_CPU(m) (size_t) ((struct seq_file *) m)->private

static struct proc_dir_entry *munch_procfs; 

static int show_munch(struct seq_file *m, const struct munch_iterator *it) {
	if (is_muncher_valid) {
		return muncher.dump_data(m, it);
	}
	return 0;
}

static void *munch_seq_start(struct seq_file *s, loff_t *pos) {
	struct munch_iterator *it = kzalloc(sizeof *it, GFP_KERNEL); // all zeros is init
	if (it != NULL && is_muncher_valid) {
		it->cpu = GET_CPU(s);
		for (size_t i = 0; i < *pos; i++) {
			muncher.move_iterator(it);
		}
		if (it->entry_index >= MUNCH_NUM_ENTRIES) {
			kfree(it);
			return NULL;
		}
		return it;
	}

        return NULL;
}

static void *munch_seq_next(struct seq_file *s, void *v, loff_t *pos) {
	struct munch_iterator *it = v;
	muncher.move_iterator(it);
	(*pos)++;

	if (it->entry_index >= MUNCH_NUM_ENTRIES) {
		return NULL;
	}

	return it;
}

static int munch_seq_show(struct seq_file *m, void *v) {
	const struct munch_iterator *it = v;
	return show_munch(m, it);
}

static void munch_seq_stop(struct seq_file *s, void *v) {
	kfree(v);
}

static const struct seq_operations munch_seq_ops = {
        .start = munch_seq_start,
        .next  = munch_seq_next,
        .stop  = munch_seq_stop,
        .show  = munch_seq_show
};

static int munch_open(struct inode *inode, struct file *file) {
        int ret = seq_open(file, &munch_seq_ops);
	if (is_muncher_valid && ret == 0) {
		size_t cpu = GET_CPU(file->private_data);
		muncher.start_dump(cpu);
	}
	return ret;
}

static int munch_release(struct inode *inode, struct file *file) {
	if (is_muncher_valid) {
		size_t cpu = GET_CPU(file->private_data);
		muncher.finalize_dump(cpu);
	}
	return seq_release(inode, file);
}

static const struct proc_ops munch_proc_ops = {
	.proc_open    = munch_open,
	.proc_read    = seq_read,
	.proc_lseek   = seq_lseek,
	.proc_release = munch_release,
};

int munch_register_procfs() {
	munch_procfs = proc_mkdir(PROCFS_NAME, NULL); 
	if (munch_procfs == NULL) { 
		pr_alert("Error:Could not initialize /proc/%s\n", PROCFS_NAME); 
		return -ENOMEM; 
	} 

	size_t cpu;
	for_each_cpu(cpu, cpu_possible_mask) {
		char file_name[10];
		memset(file_name, 0, sizeof file_name);
		sprintf(file_name, "%d", cpu);

		static struct proc_dir_entry *munch_procfs_child; 
		munch_procfs_child = proc_create_data(file_name, 0444, munch_procfs, &munch_proc_ops, (void *) cpu); 
		if (munch_procfs_child == NULL) { 
			pr_alert("Error:Could not initialize /proc/%s/%s\n", PROCFS_NAME, PROCFS_NAME); 
			return -ENOMEM; 
		}
		pr_info("/proc/%s/%s file created created\n", PROCFS_NAME, file_name); 
	}

	pr_info("/proc/%s directory created\n", PROCFS_NAME); 
	return 0; 
} 

void munch_unregister_procfs() { 
	remove_proc_subtree(PROCFS_NAME, NULL);
	pr_info("/proc/%s directory removed\n", PROCFS_NAME); 
}

bool munch_seq_has_overflowed(struct seq_file *m) {
	return seq_has_overflowed(m);
}

// helpers

size_t nr_sched_domains(size_t cpu) {
	struct rq *rq = cpu_rq(cpu);
	struct sched_domain *sd;
	size_t sd_count = 0;
	for_each_domain(rq->cpu, sd) {
		sd_count++;
	}
	return sd_count;
}

const struct sched_domain *get_sd(size_t cpu, size_t sd_index) {
	struct rq *rq = cpu_rq(cpu);
	struct sched_domain *sd;
	size_t sd_count = 0;
	for_each_domain(rq->cpu, sd) {
		if (sd_count == sd_index) {
			return sd;
		}
		sd_count++;
	}
	return NULL;
}

size_t nr_sched_groups(const struct sched_domain *sd) {
	struct sched_group *sg = sd->groups;
	size_t sg_count = 0;
	do {
		sg_count++;
		sg = sg->next;
	} while (sg != sd->groups);
	return sg_count;
}

const struct sched_group *get_sg(const struct sched_domain *sd, size_t sg_index) {
	struct sched_group *sg = sd->groups;
	size_t sg_count = 0;
	do {
		if (sg_count == sg_index) {
			return sg;
		}
		sg_count++;
		sg = sg->next;
	} while (sg != sd->groups);
	return NULL;
}
