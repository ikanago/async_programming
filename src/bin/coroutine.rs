use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll}, sync::{Mutex, Arc, mpsc::{SyncSender, Receiver}},
};

use futures::{future::BoxFuture, FutureExt, task::{ArcWake, waker_ref}};

enum HelloState {
    Hello,
    World,
    End,
}

struct Hello {
    state: HelloState,
}

impl Hello {
    fn new() -> Self {
        Hello {
            state: HelloState::Hello,
        }
    }
}

impl Future for Hello {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        match (*self).state {
            HelloState::Hello => {
                print!("Hello, ");
                (*self).state = HelloState::World;
                ctx.waker().wake_by_ref();
                Poll::Pending
            }
            HelloState::World => {
                println!("World!");
                (*self).state = HelloState::End;
                ctx.waker().wake_by_ref();
                Poll::Pending
            }
            HelloState::End => Poll::Ready(()),
        }
    }
}

struct Task {
    future: Mutex<BoxFuture<'static, ()>>,
    sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let self0 = arc_self.clone();
        arc_self.sender.send(self0).unwrap();
    }
}

struct Executor {
    sender: SyncSender<Arc<Task>>,
    receiver: Receiver<Arc<Task>>,
}

impl Executor {
    fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1024);
        Executor {
            sender: sender.clone(),
            receiver,
        }
    }

    fn get_spawner(&self) -> Spawner {
        Spawner {
            sender: self.sender.clone(),
        }
    }

    fn run(&self) {
        while let Ok(task) = self.receiver.recv() {
            let mut future = task.future.lock().unwrap();
            let waker = waker_ref(&task);
            let mut ctx = Context::from_waker(&waker);
            let _ = future.as_mut().poll(&mut ctx);
        }
    }
}

struct Spawner {
    sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(future),
            sender: self.sender.clone(),
        });

        self.sender.send(task).unwrap();
    }
}

fn main() {
    let executor = Executor::new();
    executor.get_spawner().spawn(Hello::new());
    executor.run();
}
