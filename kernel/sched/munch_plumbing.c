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
static struct proc_dir_entry *munch_procfs; 

static ssize_t procfile_read(struct file *file_pointer, char __user *buffer, 
                             size_t buffer_length, loff_t *offset) { 
    char s[13] = "muncher!\n"; 
    int len = sizeof(s); 
    ssize_t ret = len; 

    if (*offset >= len || copy_to_user(buffer, s, len)) { 
        ret = 0; 
    } else { 
        pr_info("procfile read %s\n", file_pointer->f_path.dentry->d_name.name); 
        *offset += len; 
    } 

    return ret; 
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
