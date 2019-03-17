use std::thread;
use std::thread::JoinHandle;
use std::sync::{Arc, RwLock, Condvar, Mutex};
use std::collections::VecDeque;
use lazy_static::lazy_static;

lazy_static! {
    static ref THREAD_POOL: ThreadPool = ThreadPool::new();
}

pub struct Jobs {}

#[allow(dead_code)]
impl Jobs {
    pub fn dispatch_job(job_task: JobDescriptor) {
        THREAD_POOL.push_job(job_task);
    }

    pub fn dispatch_jobs(job_tasks: &Vec<JobDescriptor>) {
        THREAD_POOL.push_job_array(&job_tasks);
    }

    pub fn wait_for_outstanding_jobs() {
        let fence = Fence::new();
        Jobs::dispatch_job(JobDescriptor::new(Arc::new(FenceJob::new(fence.clone()))));
        fence.wait();
    }

    pub fn job_queue_empty() -> bool {
        THREAD_POOL.job_queue.is_empty()
    }
}

pub trait JobTask {
    fn run(&self);
}

#[derive(Clone)]
struct Fence {
    value: Arc<(Mutex<bool>, Condvar)>,
}

impl Fence {
    fn new() -> Fence {
        Fence {
            value: Arc::new((Mutex::new(false), Condvar::new()))
        }
    }

    fn notify(&self) {
        let &(ref lock, ref condvar) = &*self.value;
        let mut notified = lock.lock().unwrap();
        *notified = true;
        condvar.notify_one();
    }

    fn wait(&self) {
        let &(ref lock, ref condvar) = &*self.value;
        let mut notified = lock.lock().unwrap();
        // wait for notifcation
        while !*notified {
            // this allows the thread to sleep and not use cycles, however it may wake up spuriously so that is why we also check "notified"
            notified = condvar.wait(notified).unwrap();
        }
    }
}

struct FenceJob {
    fence: Fence,
}

impl FenceJob {
    fn new(fence: Fence) -> FenceJob {
        FenceJob {
            fence
        }
    }
}

impl JobTask for FenceJob {
    fn run(&self) {
        self.fence.notify();
    }
}

struct ThreadPool {
    job_threads: Vec<JobThreadHandle>,
    job_queue: JobQueue,
    thread_wake_event: ThreadWakeEvent,
}

impl ThreadPool {
    pub fn new() -> ThreadPool {
        let num_cores = (num_cpus::get() - 1).max(1);
        println!("Thread pool: Spooling up {} threads", num_cores);
        
        let job_queue = JobQueue::new();
        let thread_wake_event = ThreadWakeEvent::new();
        let mut job_threads = vec![];
        for i in 0..num_cores {
            let job_thread = JobThread::new(i, job_queue.clone(), thread_wake_event.clone());
            job_threads.push(job_thread);
        }

        ThreadPool {
            job_threads,
            job_queue,
            thread_wake_event,
        }
    }

    pub fn push_job(&self, job_task: JobDescriptor) {
        self.job_queue.push(job_task);
        self.thread_wake_event.wake_threads(); // notify threads to wake
    }

    pub fn push_job_array(&self, job_tasks: &Vec<JobDescriptor>) {
        self.job_queue.push_array(&job_tasks);
        self.thread_wake_event.wake_threads();  // notify threads to wake
    }

    fn destroy(&mut self) {
        // stop each thread before waiting for them all to join
        self.job_threads.iter().for_each(|thread| thread.stop());
        // drain all threads and wait for them to join
        self.job_threads.drain(..).for_each( move |thread| thread.join());
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[derive(Clone)]
pub struct JobDescriptor{
    job: Arc<JobTask + Send + Sync + 'static>,
}

impl JobDescriptor {
    pub fn new(job: Arc<JobTask + Send + Sync + 'static>) -> JobDescriptor {
        JobDescriptor {
            job: job
        }
    }

    fn run(self) {
        self.job.run();
    }
}

#[derive(Clone)]
struct JobQueue {
    queue: Arc<RwLock<VecDeque<JobDescriptor>>>
}

#[allow(dead_code)]
impl JobQueue {
    fn new() -> JobQueue {
        JobQueue {
            queue: Arc::new(RwLock::new(VecDeque::with_capacity(10))) // initialise with some memory
        }
    }

    fn push(&self, descriptor: JobDescriptor) {
        let mut queue = self.queue.write().unwrap();
        queue.push_back(descriptor);
    }

    fn push_array(&self, descriptor_array: &Vec<JobDescriptor>) {
        let mut queue = self.queue.write().unwrap();
        for descriptor in descriptor_array {
            queue.push_back((*descriptor).clone());
        }
    }

    fn pop(&self) -> Option<JobDescriptor> {
        let mut queue = self.queue.write().unwrap();
        queue.pop_front()
    }

    fn is_empty(&self) -> bool {
        let queue = self.queue.read().unwrap();
        queue.is_empty()
    }
}

#[allow(dead_code)]
struct JobThreadHandle {
    is_running: Arc<RwLock<bool>>,
    thread_handle: JoinHandle<()>,
}

impl JobThreadHandle {
    pub fn stop(&self) {
        *self.is_running.write().unwrap() = false;
    }

    pub fn join(self) {
        self.thread_handle.join().unwrap();
    }
}

#[derive(Clone)]
struct ThreadWakeEvent {
    value: Arc<(Mutex<bool>, Condvar)>,
}

impl ThreadWakeEvent {
    fn new() -> ThreadWakeEvent {
        ThreadWakeEvent {
            value: Arc::new((Mutex::new(false), Condvar::new()))
        }
    }

    fn wake_threads(&self) {
        let &(ref lock, ref condvar) = &*self.value;
        let mut wake = lock.lock().unwrap();
        *wake = true;
        condvar.notify_all();
    }

    fn sleep_thread(&self) {
        let &(ref lock, ref condvar) = &*self.value;
        // sleep on event, this may wake spuriously but we don't really care
        let _unused = condvar.wait(lock.lock().unwrap()).unwrap();
    }
}

struct JobThread {
    thread_pool_index: usize,
    is_running: Arc<RwLock<bool>>,
    queue: JobQueue,
    wake_event: ThreadWakeEvent,
}

impl JobThread {
    fn new(thread_pool_index: usize, queue: JobQueue, wake_event: ThreadWakeEvent) -> JobThreadHandle {
        let is_running = Arc::new(RwLock::new(true));
        let job_thread = JobThread {
            thread_pool_index,
            is_running: is_running.clone(),
            queue,
            wake_event,
        };

        let thread_handle = thread::spawn( move || {
            job_thread.run();
        });

        JobThreadHandle {
            is_running, 
            thread_handle,
        }
    }

    fn run(&self) {
        println!("Job Thread {} started..", self.thread_pool_index);
        
        const SPINS_BEFORE_SLEEP: i32 = 20;
        let mut spins = 0;
        while *self.is_running.read().unwrap() {
            match self.queue.pop() {
                Some(job_descriptor) => {
                    job_descriptor.run();
                    spins = 0;
                },
                None => {
                    spins += 1;
                },
            };

            // sleep if we've no work
            if spins > SPINS_BEFORE_SLEEP {
                self.wake_event.sleep_thread();
            }

        }
        
        println!("Job Thread: {} stopped..", self.thread_pool_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct PrintJob1 {
        value: String
    }

    impl JobTask for PrintJob1 {
        fn run(&self) {
            println!("{}", self.value);
        }
    }
    #[test]
    fn test() {

        for i in 0..40 {
            let job = PrintJob1{value: format!("1st job batch: index {}", i)};
            Jobs::dispatch_job(JobDescriptor::new(Box::new(job)));
        }
        Jobs::wait_for_outstanding_jobs();

        println!("Next set of jobs incoming..");

        for i in 0..40 {
            let job = PrintJob1{value: format!("2nd job batch: index {}", i)};
            Jobs::dispatch_job(JobDescriptor::new(Box::new(job)));
        }
        Jobs::wait_for_outstanding_jobs();
    }
}
