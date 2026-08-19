use super::{Task, TaskId};
use alloc::task::Wake;
use alloc::{collections::BTreeMap, sync::Arc};
use core::task::Waker;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("queue full");
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn run_ready_tasks(&mut self) {
        while let Some(task_id) = self.task_queue.pop() {
            let task = match self.tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // 任务不存在
            };
            let waker = self
                .waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, self.task_queue.clone()));
            let mut context = Context::from_waker(waker);
            // 轮询任务
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    // 任务完成 -> 移除任务和缓存的唤醒器
                    self.tasks.remove(&task_id);
                    self.waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    /// 使用 HLT 指令让 CPU 进入低功耗挂起状态。当硬件中断触发时，CPU 离开挂起状态，
    /// 跳去执行对应的中断服务程序（ISR）。ISR 处理完毕后返回，CPU 继续执行 HLT 后的下一条指令。
    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        // 让条件判断到挂起指令成为一个原子操作，防止中间有新数据到达条件已经不成立而 hlt 仍然被执行
        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
