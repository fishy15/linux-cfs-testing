#include <linux/printk.h>
#include "karan.h"

u64 karan_counter;

void __attribute__((weak)) karan_function (void) {
        karan_counter++;
        if ((karan_counter & ((1 << 1) - 1)) == 0) {
                printk(KERN_EMERG "==k== counter is %llx\n", karan_counter);
        }
}

int __attribute__((weak)) __karan_rust_module_karan (void) {
        karan_counter++;
        if ((karan_counter & ((1 << 1) - 1)) == 0) {
                printk(KERN_EMERG "==k== [fakerust] counter is %llx\n", karan_counter);
        }
        return 0;
}

