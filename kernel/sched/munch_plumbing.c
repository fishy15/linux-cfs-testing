#include <linux/munch.h>
#include <linux/proc_fs.h>

struct munch_ops muncher;
bool is_muncher_valid = false;

void munch_flag(struct meal_descriptor *md, enum munch_flag flag) {
    if (is_muncher_valid && md != NULL) {
        muncher.munch_flag(md, flag);
    }
}

void munch64(struct meal_descriptor *md, enum munch_location_u64 location, uint64_t x) {
    if (is_muncher_valid && md != NULL) {
        muncher.munch64(md, location, x);
    }
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
#define PROCFS_BUF_SIZE 0x20000

static struct proc_dir_entry *munch_procfs; 

static int show_munch(struct seq_file *m) {
	static char procfs_buffer[PROCFS_BUF_SIZE];
	size_t cpu = (size_t) m->private;

	if (is_muncher_valid) {
		muncher.dump_data(procfs_buffer, sizeof procfs_buffer, cpu);
		seq_puts(m, procfs_buffer);
	}

	return 0;
}

static int munch_proc_show(struct seq_file *m, void *v) {
	return show_munch(m);
}

static int munch_proc_open(struct inode *inode, struct file *file) {
	return single_open(file, munch_proc_show, pde_data(inode));
}

static const struct proc_ops munch_proc_ops = { 
	.proc_open = munch_proc_open,
	.proc_read = seq_read,
	.proc_lseek = seq_lseek,
	.proc_release = single_release
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
