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
        THREAD_POOL.job_queue.push(job_task);
    }

    pub fn dispatch_jobs(job_tasks: Vec<JobDescriptor>) {
        THREAD_POOL.job_queue.push_array(job_tasks);
    }

    pub fn wait_for_outstanding_jobs() {
        let fence = Fence::new();
        Jobs::dispatch_job(JobDescriptor::new(Box::new(FenceJob::new(fence.clone()))));
        fence.wait();
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
}

impl ThreadPool {
    pub fn new() -> ThreadPool {
        let num_cores = num_cpus::get();
        println!("Thread pool: Spooling up {} threads", num_cores);
        let mut job_threads = vec![];
        let job_queue = JobQueue::new();
        for i in 0..num_cores {
            let job_thread = JobThread::new(i, job_queue.clone());
            job_threads.push(job_thread);
        }

        ThreadPool {
            job_threads,
            job_queue,
        }
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

pub struct JobDescriptor{
    job: Box<JobTask + Send + Sync + 'static>,
}

impl JobDescriptor {
    pub fn new(job: Box<JobTask + Send + Sync + 'static>) -> JobDescriptor {
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

    fn push_array(&self, descriptor_array: Vec<JobDescriptor>) {
        let mut queue = self.queue.write().unwrap();
        for descriptor in descriptor_array {
            queue.push_back(descriptor);
        }
    }

    fn pop(&self) -> Option<JobDescriptor> {
        let mut queue = self.queue.write().unwrap();
        queue.pop_front()
    }

    // low contention - will return false if queue is already locked for write
    fn is_empty(&self) -> bool {
        match self.queue.try_read() {
            Ok(queue) => {
                queue.is_empty()
            },
            Err(_) => {
                false
            },
        } 
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

struct JobThread {
    thread_pool_index: usize,
    is_running: Arc<RwLock<bool>>,
    queue: JobQueue,
}

impl JobThread {
    fn new(thread_pool_index: usize, queue: JobQueue) -> JobThreadHandle {
        let is_running = Arc::new(RwLock::new(true));
        let job_thread = JobThread {
            thread_pool_index,
            is_running: is_running.clone(),
            queue
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
        
        while *self.is_running.read().unwrap() {
            match self.queue.pop() {
                Some(job_descriptor) => {
                    job_descriptor.run();
                },
                None => {
                    // TODO(SS): Spin for a bit then got to sleep on a condvar
                },
            };
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
