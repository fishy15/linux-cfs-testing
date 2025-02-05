#include <linux/munch.h>

struct munch_ops muncher;
bool is_muncher_valid = false;

void munch64 (uint64_t x) {
    if (is_muncher_valid) {
        muncher.munch64(x);
    }
}

void set_muncher (struct munch_ops *m) {
    memcpy(&muncher, m, sizeof(struct munch_ops));
    is_muncher_valid = true;
}
