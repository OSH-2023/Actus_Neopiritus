# MLFQ算法原理、实现及测试

`多级队列反馈调度(Multi-Level Feedback Queue, MLFQ)`算法在1962年首次被提出，用于`兼容分时操作系统（Compatible Time-Sharing System, CTSS）`。在这种算法注重的场景下，既有交互式短任务，又有CPU密集型的长任务。因此，算法的目标是既要优化交互式任务的**响应时间**，又要让CPU密集型任务有足够长的时间运行，一定程度上降低**周转时间**。

## 原理

首先，考虑如果我们已经知道了所有进程的特征，应该怎么做。很显然，只要存在准备好的交互式进程，就要优先响应它们；而对于交互式进程，应该采用时间片轮转(RR)的方式调度，以降低响应时间。对计算密集型进程，为公平和统一起见，不妨也采用RR算法；但这时不需要低响应时间，因此我们可以适当增加时间片大小。这样，我们就抽象出了一个优先级的模型：交互式进程设为高优先级，CPU密集型设为低优先级；同优先级采用RR调度，且时间片大小与优先级负相关（规则1、2）。当然，交互式或计算型任务并不是简单的二分类，因此我们可以适当增加优先级的数量。这其实就是基本的固定优先级的多级无反馈队列调度。

但很显然，我们并不能一开始知道每个进程的特点；同时，交互式/计算式也不是固定的性质：一个程序完全可以一段时间内表现为交互式，一段时间表现为计算型任务。因此，*Corbato*提出了MLFQ这种调度方式。这种算法的思想是**以史为鉴**，通过进程过去一段时间内的表现，估计进程的特征，据此动态调整其优先级。其基于上面的多级队列调度，且基本规则如下（规则3、4）：

- 进程进入系统时，放在最高优先级（最上层队列）。
- 如果进程用完整个时间片，降低其优先级。否则（时间片结束前主动让出CPU），保持在相同优先级不变。

这样，交互式进程会停留在顶层，获得最快的响应时间，而长进程降到底层，获得更多的CPU时间，实现了我们的目标。另外，考虑当没有交互式进程的特殊情况，所有进程会同时下降其优先级，短进程一段时间内运行完毕，而长进程降到底层轮转运行；由于高优先级时间片较小，这其实一定程度上近似了SJF调度，使得周转时间的指标也得到了优化。

目前的调度仍然存在两个问题。首先，如果存在大量交互式进程，就会出现饥饿问题；另外，程序从计算式进程转到交互式进程的情况并没有被考虑到。我们只要加入最后一个规则：每经过一段时间 S，将系统中所有工作重新加入最高优先级队列（规则5）。这样，两个问题都得到了解决。

最后，调度最难的部分其实仍然是设置一个合理的参数。虽然比起教材上抽象的描述，我们已经确定了十分具体的调度方案，但仍然有三个重要的参数需要设置：队列数量，时间片与优先级关系，以及优先级重置时间。大多数MLFQ调度的实现都动态配置这三个参数，例如Solaris提供了一组管理员可修改的表来决定它们。该表默认有60层队列， 时间片长度从20毫秒（最高优先级），到几百毫秒（最低优先级），每一秒左右提升一次进程的优先级。

## 实现

我们考虑如何为ArceOS这一系统加入MLFQ调度方式。ArceOS目前在最顶层用户库`libax`中模仿`std::thread`，提供了`thread`这一API，功能为`axtask`模块的封装；而`axtask`涉及到调度的部分会调用`scheduler`模块的接口，其中可以选择已经实现的`fifo`，`rr`等调度器。因此，要实现新的调度器，只需在`scheduler`模块中加入新的调度器代码，并接入到上层API即可。

![img](../researches/img/ArceOS.svg)

为了方便开发、维护等，ArceOS将调度器抽象为[`BaseScheduler`](http://rcore-os.cn/arceos/scheduler/trait.BaseScheduler.html)这一trait，只要为调度器类实现这一trait，即可接入调度接口。下面逐条分析该trait的实现：

- `type SchedItem;`

  - 调度器中存放的不仅有任务，还包括与任务相对应的信息。例如对RR调度器，需要额外记录目前剩余的时间片，以判断时间是否耗尽。对MLFQ，我们需要记录任务的优先级，同时由于同级采用RR调度，也需要剩余时间片的信息。因此我们的任务定义如下：

    ``` rust
    pub struct MLFQTask<T, const LEVEL_NUM: usize, const BASE_TIME: usize, const RESET_TIME: usize> {
        inner: T,
        priority: AtomicIsize,
        remain_time: AtomicIsize,
    }
    ```

- `fn init(&mut self);`

  - 初始化调度器。本实现不需要额外的初始化，因此该函数为空。

- `fn add_task(&mut self, task: Self::SchedItem);`

  - 向调度器中加入任务。我们将其放入对应优先级的队列即可。

    ``` rust
    self.ready_queue[task.get_prio() as usize].push_back(task);
    ```

- `fn remove_task(&mut self, task: &Self::SchedItem) -> Option<Self::SchedItem>;`

  - 从调度器中移除任务，返回其所有权。只要从对应队列中移除即可。

    ``` rust
    self.ready_queue[task.get_prio() as usize]
                .iter()
                .position(|t| Arc::ptr_eq(t, task))
                .and_then(|idx| self.ready_queue[task.get_prio() as usize].remove(idx))
    ```

- `fn pick_next_task(&mut self) -> Option<Self::SchedItem>;`

  - 从调度器获取下一个要执行的任务。根据前两条规则，我们选择优先级最高的有任务的队列，返回队首任务。

    ``` rust
    // Rule 1: If Priority(A) > Priority(B), A runs (B doesn’t).
    // Rule 2: If Priority(A) = Priority(B), A & B run in round-robin fashion using the time slice (quantum length) of the given queue.
    for i in 0..self.ready_queue.len() {
        if !self.ready_queue[i].is_empty() {
            return self.ready_queue[i].pop_front();
        }
    }
    return None;
    ```

- `fn put_prev_task(&mut self, prev: Self::SchedItem, preempt: bool);`

  - 放回之前取出的任务。其中参数`preempt`代表该任务是否是被抢占的，如果是被抢占的，它有机会被放到队首。根据规则4，如果用完了时间片，我们对其降级。

    ``` rust
    // Rule 4: Once a job uses up its time allotment at a given level its priority is reduced.
    let rem = prev.get_remain();
    let prio = prev.get_prio();
    if rem <= 0 {
        prev.prio_demote();
        self.ready_queue[prev.get_prio() as usize].push_back(prev);
    } else if preempt {
        prev.reset_time();
        self.ready_queue[prio as usize].push_front(prev);
    } else {
        prev.reset_time();
        self.ready_queue[prio as usize].push_back(prev);
    }
    ```

- `fn task_tick(&mut self, current: &Self::SchedItem) -> bool;`

  - 通知调度器经过了一次时钟中断，调度器返回是否需要重新调度。根据规则2和5，需要重新调度的有两种情况：目前任务用完了时间片，或者到了重置优先级的时间。

    ```rust
    // Rule 5: After some time period S, move all the jobs in the systemto the topmost queue.
    if self.reset_remain_ticks.fetch_sub(1, Ordering::Release) <= 1 {
        self.reset_remain_ticks.store(RESET_TIME as isize, Ordering::Release);
        let mut new_queue : VecDeque<Arc<MLFQTask<T, LEVEL_NUM, BASE_TIME, RESET_TIME>>> = VecDeque::new();
        for i in 0..LEVEL_NUM {
            while let Some(item) = self.ready_queue[i].pop_front() {
                item.reset_prio();
                new_queue.push_back(Arc::clone(&item));
            }
        }
        self.ready_queue[0] = new_queue;
        for i in 1..LEVEL_NUM {
            self.ready_queue[i].clear();
        }
        current.reset_prio();
        // need reschedule
        return true;
    }
    // need reschedule
    current.tick() <= 1
    ```

- `fn set_priority(&mut self, task: &Self::SchedItem, prio: isize) -> bool;`

  - 设置当前任务的优先级，并返回是否成功。我们不允许用户手动设置优先级，所以只需一直返回false。



## 测试

我们对实现的MLFQ调度做简单的测试。

首先，ArceOS为调度器提供了一个基础速度的评测，可以直接使用，测量指标为从调度器中的pick速度和remove速度。对不同调度算法的测试数据如下：

- | 调度算法 | pick速度          | remove速度         |
  | -------- | ----------------- | ------------------ |
  | MLFQ     | 61 $ns/task$       | 62.827 $\mu s/task$ |
  | FIFO     | 44 $ns/task$       | 56 $ns/task$        |
  | RR       | 28 $ns/task$       | 66.649 $\mu s/task$ |
  | CFS      | 1.392 $\mu s/task$ | 862 $ns/task$       |

- 可以看到我们实现的MLFQ基础速度上相比RR慢的并不多。

另外，ArceOS已经存在一些基础的测试，稍作修改后就可以直接使用。

- parallel：简单的并行测试

  - 该测试简单地启动16个任务，最后将结果聚在一起并与正确结果比对，用于判断所有任务是否能成功结束，且结果是否正确。
  - 测试成功
    ```
    part 15: TaskId(19) finished
    part 0: TaskId(4) finished
    part 1: TaskId(5) finished
    part 2: TaskId(6) finished
    part 3: TaskId(7) finished
    part 4: TaskId(8) finished
    part 5: TaskId(9) finished
    part 6: TaskId(10) finished
    part 7: TaskId(11) finished
    part 8: TaskId(12) finished
    part 9: TaskId(13) finished
    part 10: TaskId(14) finished
    part 11: TaskId(15) finished
    part 12: TaskId(16) finished
    part 13: TaskId(17) finished
    part 14: TaskId(18) finished
    sum = 61783189038
    Parallel summation tests run OK!
    ```

- priority：优先级测试

  - 在该测试中，存在四个较短的任务，和一个长任务同时运行，且短任务被赋予不同的优先级。由于我们的调度器并不允许手动指定优先级，因此不能进行优先级测试，但该测试可用于观察近似SJF的程度。

  - 我们与已实现的其它算法对比：（时间均以毫秒计算）

    | 调度算法 | Task 0 | Task 1  | Task 2  | Task 3  | Task 4 （长任务） |
    | -------- | ------ | ------- | ------- | ------- | ----------------- |
    | **MLFQ** | **68** | **138** | **207** | **277** | **347**           |
    | RR       | 268    | 287     | 308     | 328     | 347               |
    | FIFO     | 69     | 139     | 209     | 279     | 348               |

  - 可以看到该例中MLFQ对SJF有较好的近似。（FIFO由于顺序与长短正好相同，调度也与SJF相同）

- 此外还有sleep和yield两个测试，用于测试这两个功能的正确性，我们的调度均通过了测试，在此不再展开。

此外，为了更详细地评测调度算法，我们添加了更多测试：

- realtime：响应时间测试

  - 有四个**交互式**（运行极短时间后主动yield，大量循环）短进程和一个长进程同时运行，判断长进程的运行对短进程的响应时间的影响。

  - 任务的总响应时间如下表：

  - | 调度算法 | Task 0  | Task 1  | Task 2  | Task 3  | Task 4 （长任务） |
    | -------- | ------- | ------- | ------- | ------- | ----------------- |
    | **MLFQ** | **402** | **395** | **375** | **190** | **1771**          |
    | RR       | 1745    | 1736    | 1716    | 1705    | 1680              |
    | FIFO     | 1744    | 1735    | 1717    | 1706    | 1682              |

  - 这里RR的表现相对并不好，原因是交互式进程**每次**都要等待长进程运行一个时间片；而MLFQ是在没有就绪高优先级任务之后，才运行低优先级任务。

- starvation：饥饿测试

  - 有四个不停止的交互式任务，观察在该情况下长任务完成的用时。

  - | 调度算法 | 长任务完成时间                     |
    | -------- | ---------------------------------- |
    | MLFQ     | 1713                               |
    | RR       | 1691                               |
    | FIFO     | 1678                               |
    | CFS      | （与优先级设置有关）2000-15000不等 |

  - 可以看到本测试中MLFQ对饥饿控制的较好，但这并不是无代价的，与参数的设置有很大关系。

## 参考资料

1. [进程调度-rCore-Tutorial-Book](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter5/4scheduling.html)
2. [Multilevel feedback queue](https://en.wikipedia.org/wiki/Multilevel_feedback_queue)
3. [Scheduling: The Multi-Level Feedback Queue](https://pages.cs.wisc.edu/~remzi/OSTEP/cpu-sched-mlfq.pdf)