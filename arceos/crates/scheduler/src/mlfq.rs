use alloc::{collections::VecDeque, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicIsize, Ordering};
use core::ops::Deref;

use crate::BaseScheduler;

/// A task wrapper for the [`MLFQScheduler`].
///
/// It add a priority for scheduling between levels.
/// It add a time slice counter to use in round-robin scheduling within level.
pub struct MLFQTask<T, const LEVEL_NUM: usize, const BASE_TIME: usize, const RESET_TIME: usize> {
    inner: T,
    priority: AtomicIsize,
    remain_time: AtomicIsize,
}

impl<T, const LEVEL_NUM: usize, const BASE_TIME: usize, const RESET_TIME: usize> MLFQTask<T, LEVEL_NUM, BASE_TIME, RESET_TIME> {
    /// Creates a new [`MLFQTask`] from the inner task struct.
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            priority: AtomicIsize::new(0 as isize), // Rule 3: new job is placed at the highest priority
            remain_time: AtomicIsize::new(BASE_TIME as isize),
        }
    }

    /// Load task priority
    pub fn get_prio(&self) -> isize {
        self.priority.load(Ordering::Acquire)
    }
    
    /// Sub remain time by 1, used when time tick
    pub fn tick(&self) -> isize {
        self.remain_time.fetch_sub(1, Ordering::Release)
    }

    /// Load remain time tick
    pub fn get_remain(&self) -> isize {
        self.remain_time.load(Ordering::Acquire)
    }

    /// Reset remain time
    pub fn reset_time(&self) {
        self.remain_time.store((BASE_TIME as isize) << self.priority.load(Ordering::Acquire), Ordering::Release);
        // Rule: higher-priority queues get shorter time slices (they are more likely to be interactive jobs)
    }

    /// Reset priority (Rule 5)
    pub fn reset_prio(&self) {
        self.priority.store(0, Ordering::Release);
        self.remain_time.store(BASE_TIME as isize, Ordering::Release);
    }

    /// Demote priority by 1 (Rule 4)
    pub fn prio_demote(&self) -> isize {
        let mut current_prio = self.priority.fetch_add(1, Ordering::Release) + 1;
        if current_prio == LEVEL_NUM as isize {
            self.priority.store(LEVEL_NUM as isize - 1, Ordering::Release);
            current_prio = LEVEL_NUM as isize - 1;
        }
        self.remain_time.store((BASE_TIME as isize) << current_prio, Ordering::Release);
        current_prio as isize
    }

    /// Returns a reference to the inner task struct.
    pub const fn inner(&self) -> &T {
        &self.inner
    }
}

impl<T, const LEVEL_NUM: usize, const BASE_TIME: usize, const RESET_TIME: usize> Deref for MLFQTask<T, LEVEL_NUM, BASE_TIME, RESET_TIME> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A simple Multi-Level Feedback Queue (MLFQ) preemptive scheduler.
///
/// Same as [`RRScheduler`], it uses [`VecDeque`] as the ready queue. So it may
/// take O(n) time to remove a task from the ready queue.
/// 
/// Main Reference: [`WISC`]
/// 
/// Five Rules:
/// - Rule 1: If Priority(A) > Priority(B), A runs (B doesn’t).
/// - Rule 2: If Priority(A) = Priority(B), A & B run in round-robin fashion using the time slice (quantum length) of the given queue.
/// - Rule 3: When a job enters the system, it is placed at the highest priority (the topmost queue).
/// - Rule 4: Once a job uses up its time allotment at a given level, its priority is reduced (i.e., it moves down one queue).
/// - Rule 5: After some time period S, move all the jobs in the systemto the topmost queue.
/// 
/// [`WISC`]: https://pages.cs.wisc.edu/~remzi/OSTEP/cpu-sched-mlfq.pdf
/// [`RRScheduler`]: crate::RRScheduler
pub struct MLFQScheduler<T, const LEVEL_NUM: usize, const BASE_TIME: usize, const RESET_TIME: usize> {
    ready_queue: Vec<VecDeque<Arc<MLFQTask<T, LEVEL_NUM, BASE_TIME, RESET_TIME>>>>,
    reset_remain_ticks: AtomicIsize,
}

impl<T, const LEVEL_NUM: usize, const BASE_TIME: usize, const RESET_TIME: usize> MLFQScheduler<T, LEVEL_NUM, BASE_TIME, RESET_TIME> {
    /// Creates a new empty [`MLFQScheduler`].
    pub fn new() -> Self {
        assert!(LEVEL_NUM > 0);
        let mut ready_queue = Vec::new();
        for _i in 0..LEVEL_NUM {
            ready_queue.push(VecDeque::new());
        }
        Self {
            ready_queue,
            reset_remain_ticks: AtomicIsize::new(RESET_TIME as isize),
        }
    }

    /// get the name of scheduler
    pub fn scheduler_name() ->  &'static str{
        "Multi-Level Feedback Queue Scheduler"
    }
}

impl<T, const LEVEL_NUM: usize, const BASE_TIME: usize, const RESET_TIME: usize> BaseScheduler for MLFQScheduler<T, LEVEL_NUM, BASE_TIME, RESET_TIME> {
    type SchedItem = Arc<MLFQTask<T, LEVEL_NUM, BASE_TIME, RESET_TIME>>;

    fn init(&mut self) {}

    fn add_task(&mut self, task: Self::SchedItem) {
        self.ready_queue[task.get_prio() as usize].push_back(task);
    }

    fn remove_task(&mut self, task: &Self::SchedItem) -> Option<Self::SchedItem> {
        self.ready_queue[task.get_prio() as usize]
            .iter()
            .position(|t| Arc::ptr_eq(t, task))
            .and_then(|idx| self.ready_queue[task.get_prio() as usize].remove(idx))
    }

    fn pick_next_task(&mut self) -> Option<Self::SchedItem> {
        // Rule 1: If Priority(A) > Priority(B), A runs (B doesn’t).
        // Rule 2: If Priority(A) = Priority(B), A & B run in round-robin fashion using the time slice (quantum length) of the given queue.
        for i in 0..self.ready_queue.len() {
            if !self.ready_queue[i].is_empty() {
                return self.ready_queue[i].pop_front();
            }
        }
        return None;
    }

    fn put_prev_task(&mut self, prev: Self::SchedItem, preempt: bool) {
        // Rule 4: Once a job uses up its time allotment at a given level its priority is reduced.
        if Arc::clone(&prev).get_remain() <= 0 {
            prev.prio_demote();
            self.ready_queue[Arc::clone(&prev).get_prio() as usize].push_back(prev);
        } else if preempt {
            prev.reset_time();
            self.ready_queue[Arc::clone(&prev).get_prio() as usize].push_front(prev);
        } else {
            prev.reset_time();
            self.ready_queue[Arc::clone(&prev).get_prio() as usize].push_back(prev);
        }
    }

    fn task_tick(&mut self, current: &Self::SchedItem) -> bool {
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
    }

    /// User cannot set priority
    fn set_priority(&mut self, _task: &Self::SchedItem, _prio: isize) -> bool {
        false
    }
}