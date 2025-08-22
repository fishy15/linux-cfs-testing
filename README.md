# linux-cfs-testing

A fork of Linux 6.13 with the `munch` subsystem for tracing codepaths in the Linux scheduler.
For more details on how it works
and its purpose,
refer to [this report](https://apps.cs.utexas.edu/apps/sites/default/files/tech_reports/load_balance_cx_synthesis.pdf).
The goal is to store which values the load balancer uses to make its decision,
and to mark unseen values as null.

## Usage

### Compiling the kernel

For convenience,
a kernel configuration that enables Rust support
and other necessary features
can be found at `./myconfig`.
The script `./setup.sh` handles installing Rust,
and `./compile.sh` compiles the kernel with the right configuration.
The kernel is compiled into `../kbuild`.

### Collecting data

To start collecting data,
load the Rust module into the kernel
and run a Linux VM with this kernel.
`./run.sh` can be used to more easily run virtual machines with different configurations.
Assuming there is some Debian image at `../vm/d.q`,
running `K=1 ./run.sh` will start the virtual machine with the kernel loaded.
It can be configured using the following environment variables:
- `TOPOLOGY`: two possible options
  - enter `N`, where `N` is some positive integer, for a flat topology with `N` cores
  - enter `16-tiered` for a topology of four groups of four cores
- `SSH`: change the SSH port into the virtual machine (by default, it is 2222).

Note that the `K=1` is necessary.
(The intention was that not including the flag would run the VM with the default kernel,
but there are some configuration issues).
To add new topologies,
please modify the `./run.sh` file.

### Retrieving results

To retrieve the results,
copy the data from `/proc/munch/<cpu-number>`
from the VM.
This stores the traces for the values collected on the past `MUNCH_NUM_ENTRIES` instances of load balance.
This value is set to 256 in `./kernel/sched/munch_plumbing.c`.
The output format is in JSON.

## Development

The following files are likely the files which need to be modified:
- `./include/linux/munch.h` — C interface into the Rust module.
- `./copy-file.sh` — helper script to copy munch data back to the host machine.
- `./include/linux/sched/fair_enums.h` — enums that were originally local to `./kernel/sched/fair.c`, 
  but moved to a global header file to be accessible in the Rust module.
- `./kernel/sched/fair.c` — original implementation of the scheduler and load balancer,
  with function calls to `munch` to copy values.
- `./kernel/sched/munch_plumbing.c` — implementation of the C interface, directly calls Rust code.
   Additionally sets up the `proc` file system for `munch`.
- `./rust/kernel/munch_ops.rs` — Rust vtable to allow being called from C
- `./myconfig` — kernel configuration file with features enabled for this project.
- `./run.sh` — script for helping run a VM loaded with this kernel.
- `./rust/bindgen_parameters` — contains a list of which enums should be represented as Rust enums instead of integers.
- `./swk.c` — C script for running a task on a VM and collecting `munch` data from it.
   Mainly used in [this CloudLab configuration](https://github.com/fishy15/cfs-testing-cloudlab-profile).

To log a new value:
1. Add a new variable and its type into the logging struct.
   Likely, this will be in `LBIPerCpu` (information stored per-cpu),
   `LBIPerSchedDomainInfo` (information stored per scheduler domain),
   and `lBIPerSchedGroup` (information stored per CPU group).
3. Check if there is some Rust function assigning to that type and in that location (cpu/domain/group).
   For example,
   `set_value_bool_cpu` sets a boolean per-cpu value.
   As a convention,
   if the function sets a variable at the scheduler domain level,
   then the `_sd` is not specified.
   If it exists already,
   then skip this step.
   Otherwise,
   the following need to be created:
   - enum for that type and location (`./include/linux/munch.h`)
   - name of the enum to be Rustified (`./rust/bindgen_parameters`)
   - C function definition in the `munch_ops` vtable for that type and location (`./include/linux/munch.h`)
   - C function definition for that type and location (`./include/linux/munch.h`)
   - implementation of the above function (`./kernel/sched/munch_plumbing.c`)
   - Rust function definition in the `MunchOps` trait for that type and location (`./rust/kernel/munch_ops.rs`)
   - implementation of the function in the `impl<T: MunchOps> MunchOpsVTable<T>` block (`./rust/kernel/munch_ops.rs`)
   - implementation of the trait in the `impl MunchOps for RustMunch` block (`./kernel/sched/munch.rs`)
   - global Rust function that actually assigns the value, similar to `set_value_bool_cpu`) (`./kernel/sched/munch.rs`)
4. Create a new enum value in `./include/linux/munch.h` that corresponds to the new variable you are storing.
5. In the global Rust function that actually assigns the value,
   add a branch to the match statement that assigns the value.

Initialization and outputting the new variables should already be handled by the macros used.
For more details,
there are comments in the files explaining certain segments of code.
