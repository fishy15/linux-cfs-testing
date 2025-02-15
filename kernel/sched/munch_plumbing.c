#include <linux/munch.h>

struct munch_ops muncher;
bool is_muncher_valid = false;

void munch64(size_t meal_descriptor, enum munch_location location, uint64_t x) {
    if (is_muncher_valid && meal_descriptor != -1) {
        muncher.munch64(meal_descriptor, location, x);
    }
}

void set_muncher (struct munch_ops *m) {
    memcpy(&muncher, m, sizeof(struct munch_ops));
    is_muncher_valid = true;
}

size_t open_meal() {
    if (is_muncher_valid) {
        return muncher.open_meal();
    }
    return -1;
}
