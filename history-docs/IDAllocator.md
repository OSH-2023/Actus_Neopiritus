# ID 分配器

## ID 分配器简介

ID 分配器（ID Allocator）是一个用于管理唯一标识符（ID）的分配与回收的工具。其功能主要包括：

- 分配唯一 ID：每次请求一个新的 ID 时，ID 分配器会生成一个尚未分配的唯一 ID。这个 ID 通常是一个自增的整数。
- 释放 ID：当某个 ID 不再使用时，可以将其归还给 ID 分配器。这样，在需要新的 ID 时，分配器可以重用这些已释放的 ID，从而减少 ID 耗尽的风险。
- 确保 ID 唯一性：ID 分配器需要确保在任何时候，分配出去的 ID 都是唯一的。

在操作系统中，ID 分配器主要用于管理系统资源的标识符，确保了每个资源能够被准确识别和管理。其有如下应用：

- 进程和线程 ID 分配：操作系统为每个进程和线程分配一个唯一的 ID，称为进程 ID（PID）和线程 ID（TID）。这些 ID 用于标识和管理各种进程和线程，例如调度、同步和资源分配。
- 文件描述符分配：操作系统为每个打开的文件和套接字分配一个唯一的整数 ID，称为文件描述符（File Descriptor, FD）。文件描述符用于表示打开的文件或套接字，并通过系统调用（如 read、write 和 close）进行操作。
- 内存页表项分配：在虚拟内存管理中，操作系统需要为每个内存页分配一个唯一的虚拟地址。为了实现这一目标，操作系统使用 ID 分配器来分配和管理虚拟地址空间中的页面。

操作系统中的 ID 分配器通常需要具备高性能、低开销和易于管理的特性。为了实现这些目标，操作系统内核可能会采用各种优化策略，如分层管理、缓存和预分配。这些策略有助于提高 ID 分配和回收的速度，降低资源耗尽的风险，并简化资源管理任务。[kernel.org](https://www.kernel.org/doc/html/latest/core-api/idr.html) 中介绍了 Linux Kernel ID 分配器提供的 API。

## ID 分配器实现

在 arceOS 中，ID 分配器源码位于 `crates/allocator/src`。其中 ID 分配器需要实现 6 个功能。

### `alloc_id` 分配 ID

```rust
/// Allocate contiguous IDs with given count and alignment.
fn alloc_id(&mut self, count: usize, align_pow2: usize) -> AllocResult<usize>;
```

`alloc_id` 方法的目的是分配 `count` 个连续的 ID，并确保第一个 ID 满足对齐要求。其设计思路如下：

1. 参数检查：首先检查输入参数 `count` 是否合法，其不应为 0。此外，`let align = 1 << align_pow2`。
1. 计算第一个 ID：`base_id` 是第一个可能满足对齐要求的 ID。我们可以将 `next_id` 向上舍入到最接近的 `align` 的倍数，得到 `base_id`。这可以通过以下公式实现：(`next_id` + `align` - 1) / `align` * `align`。
1. 如果 `base_id` 在超过 ID 的最大范围，则分配失败，返回 `AllocResult::Failed`。
1. 分配 ID：从 `base_id` 起便是 `count` 个连续的空闲 ID。我们需要将原 `next_id` 到 `base_id` - 1 间的 ID 加入至 `free_ids` 中，以表明它们尚未分配（可以用哈希表实现这个集合，修改/查询时间均为 $O(1)$）。同时，我们需要更新 `next_id` 的值，确保它大于已分配的最大 ID。
1. 分配成功，返回基址 ID 的值 `AllocResult::Ok(base_id)`。

这种设计方法寻找空闲 ID 时，无需任何循环遍历，这在大多数情况下效率较高。然而，在极端情况下，如果 ID 空间非常分散，这可能导致 `free_ids` 很大，增加哈希表的常数时间。针对这种情况，可以考虑使用更高级的数据结构（如线段树）来加速连续 ID 的搜索和分配过程；但对于一般情况，线段树常数较大，没有较大价值。

### `dealloc_id` 释放 ID

```rust
/// Deallocate contiguous IDs with given position and count.
fn dealloc_id(&mut self, start_id: usize, count: usize);
```

`dealloc_id` 方法的目的是释放 `count` 个连续的 ID。其设计思路如下：

1. 释放连续的 `count` 个 ID 时，我们将 `start_id` 到 min(`next_id`, `start_id` + `count`) - 1 的每个 ID，将其插入至 `free_ids` 集合中。这表明该 ID 现在是空闲的，可以在下次分配时被重用。
1. 随后，逐步尝试递减 `next_id`，直至 `next_id` 为 0 或 `next_id` - 1 不在 `free_ids` 中。`next_id` 递减时，也要将相应的 `next_id` - 1 从 `free_ids` 中移除。

### `is_allocated` 检查 ID

```rust
/// Whether the given `id` was allocated.
fn is_allocated(&self, id: usize) -> bool;
```

`is_allocated` 方法的目的是检查 ID 是否已经分配。其设计思路如下：

1. 如果 `id` 大于等于 `next_id`，则说明它尚未分配。在这种情况下，函数应返回 `false`。
1. 如果 `id` 小于 `next_id`，我们需要检查它是否在 `free_ids` 集合中。如果在，说明它已经被释放，因此函数应返回 `false`；否则，说明它已被分配，函数应返回 `true`。

### `alloc_fixed_id` 分配固定 ID

```rust
/// Mark the given `id` has been allocated and cannot be reallocated.
fn alloc_fixed_id(&mut self, id: usize) -> AllocResult;
```

`alloc_fixed_id` 方法的目的是将给定的 `id` 标记为已分配，确保它不会在后续分配中被重用。其设计思路如下：

1. 先调用 `is_allocated` 方法判断 `id` 是否已分配。若 `id` 已分配，则返回 `AllocResult::Failed`。
1. 若 `id` 未分配，则分配 `id`，并返回 `AllocResult::Ok`：
    - 如果 `id` 小于 `next_id`，则将其从 `free_ids` 集合中移除。
    - 如果 `id` 大于等于 `next_id`，则需要将 `next_id` 更新为 `id` + 1，以确保后续分配的 ID 不会与 `id` 重复。另外，需要将原 `next_id` 到 `id` - 1 的 ID 加入至 `free_ids` 集合中。

### `size` 查询 ID 最多数目

```rust
/// Returns the maximum number of supported IDs.
fn size(&self) -> usize;
```

`size` 方法的目的是查询 ID 的最多数目（ID 的最大值 + 1）。这通常是人为规定的常量，直接返回即可。

### `used` 查询已分配的 ID 数目

```rust
/// Returns the number of allocated IDs.
fn used(&self) -> usize;
```

`used` 方法的目的是查询已分配的 ID 数目。返回值为 `next_id` - `free_ids.len()`。

### `available` 查询可用的 ID 数目

```rust
/// Returns the number of available IDs.
fn available(&self) -> usize;
```

`available` 方法的目的是查询可用的 ID 数目。返回值为 `size()` - `used()`。
