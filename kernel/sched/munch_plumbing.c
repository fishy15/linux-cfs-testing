#include <linux/munch.h>

#include <linux/proc_fs.h>

struct munch_ops muncher;
bool is_muncher_valid = false;

void munch64(struct meal_descriptor *md, enum munch_location location, uint64_t x) {
    if (is_muncher_valid) {
        muncher.munch64(md, location, x);
    }
}

void set_muncher(struct munch_ops *m) {
    memcpy(&muncher, m, sizeof(struct munch_ops));
    is_muncher_valid = true;
}

void open_meal(size_t cpu_number, struct meal_descriptor *md) {
    if (is_muncher_valid) {
        return muncher.open_meal(cpu_number, md);
    }
}

// procfs

#define PROCFS_NAME "munch" 
#define PROCFS_BUF_SIZE 0x20000

static struct proc_dir_entry *munch_procfs; 
static char procfs_buffer[PROCFS_BUF_SIZE];

static ssize_t procfile_read(struct file *file_pointer, char __user *buffer, 
                             size_t buffer_length, loff_t *offset) { 
	// dump all info at once, so offset should never be >0
	if (*offset > 0) {
		return 0;
	}

	if (is_muncher_valid) {
		ssize_t length = muncher.dump_data(buffer, buffer_length);
		*offset = length;
		return length;
	}

	return 0;
} 

static const struct proc_ops proc_file_fops = { 
	.proc_read = procfile_read, 
}; 

int munch_register_procfs() {
	munch_procfs = proc_create(PROCFS_NAME, 0644, NULL, &proc_file_fops); 
	if (munch_procfs == NULL) { 
		pr_alert("Error:Could not initialize /proc/%s\n", PROCFS_NAME); 
		return -ENOMEM; 
	} 

	pr_info("/proc/%s created\n", PROCFS_NAME); 
	return 0; 
} 

void munch_unregister_procfs() { 
	proc_remove(munch_procfs); 
	pr_info("/proc/%s removed\n", PROCFS_NAME); 
}
