use crate::print;
use crate::println;
use alloc::string::String;
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::future;
use futures_util::stream::Stream;
use futures_util::stream::StreamExt;
use futures_util::task::AtomicWaker;
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};

/// ArrayQueue::new 会执行堆分配，而这在编译时还无法实现，我们无法直接初始化静态变量。
/// 为此，使用了 conquer_once crate 的 OnceCell 类型，它能安全地实现静态值的一次性初始化.
/// OnceCell 确保初始化操作不会发生在中断处理程序中，从而防止中断处理程序执行堆分配操作。
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

static WAKER: AtomicWaker = AtomicWaker::new();

// 仅对 lib 可用
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        // 队列尚未初始化
        println!("WARNING: scancode queue uninitialized");
    }
}

pub fn init() {
    SCANCODE_QUEUE
        .try_init_once(|| ArrayQueue::new(100))
        .expect("ScancodeStream::new should only be called once");
}

pub struct ScancodeStream {
    _private: (), // 防止从模块外部构造该结构体
}

impl ScancodeStream {
    pub fn new() -> Self {
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    /// 典型实现方式：直接读，能读到就返回ready，不能读到则更新waker，然后返回peding，
    /// 不会记录状态。能否读到即是状态。
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE.try_get().expect("not initialized");
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        // 双重验证，防止更新waker的间隙又有新数据到了，导致有效数据可能再也无法被读取到
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

pub fn keyboard_stream() -> impl Stream<Item = char> {
    let scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );
    scancodes.filter_map(move |scancode| {
        // 扫描码转化为KeyEvent，KeyEvent 包括了触发本次中断的按键信息，以及子动作是按下还是释放
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            // KeyEvent转换为人类可读的字符
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => return future::ready(Some(character)),
                    DecodedKey::RawKey(_) => {}
                }
            }
        }
        future::ready(None)
    })
}

pub async fn print_keypresses() {
    let mut input = keyboard_stream();

    while let Some(scancode) = input.next().await {
        print!("{}", scancode)
    }
}

pub async fn readln() -> String {
    let mut input = keyboard_stream();
    let mut s = String::with_capacity(8);
    while let Some(c) = input.next().await {
        print!("{}", c);
        if c == '\n' {
            return s;
        }
        s.push(c);
    }
    s
}
