use std::thread;
use std::thread::JoinHandle;
use std::sync::Arc;
use parking_lot::{RwLock, Condvar, Mutex};
use std::collections::VecDeque;
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::UnsafeCell;

lazy_static! {
    static ref THREAD_POOL: ThreadPool = ThreadPool::new();
}

pub struct Jobs {}

#[allow(dead_code)]
impl Jobs {
    pub fn dispatch_job(job_task: Arc<RwLock<JobTask + Send + Sync + 'static>>) -> Arc<JobCounter>  {
        THREAD_POOL.push_job(job_task)
    }

    pub fn dispatch_jobs(job_tasks: &Vec<Arc<RwLock<JobTask + Send + Sync + 'static>>>) -> Arc<JobCounter> {
        THREAD_POOL.push_job_array(&job_tasks)
    }

    pub fn wait_for_counter( job_counter: &JobCounter, value: usize) {
        job_counter.wake_on_value(value);
    }

    pub fn job_queue_empty() -> bool {
        THREAD_POOL.job_queue.is_empty()
    }
}

pub trait JobTask {
    fn run(&mut self);
}

pub struct JobCounter {
    condvar: Condvar,
    counter: Mutex<AtomicUsize>
}

impl JobCounter {
    fn new(count: usize) -> JobCounter {
        JobCounter {
            condvar: Condvar::new(),
            counter: Mutex::new(AtomicUsize::new(count)),
        }
    }

    fn decrement(&self) {
        let counter = self.counter.lock();
        counter.fetch_sub(1, Ordering::SeqCst);
        self.condvar.notify_one();
    }

    fn wake_on_value(&self, value: usize)  {
        let mut counter = self.counter.lock();
        while counter.compare_and_swap(value, 1, Ordering::Acquire) != value {
            self.condvar.wait(&mut counter);
        }
    }
}

struct ThreadPool {
    job_threads: Vec<JobThreadHandle>,
    job_queue: JobQueue,
    thread_wake_event: ThreadWakeEvent,
}

impl ThreadPool {
    pub fn new() -> ThreadPool {
        let num_cores = (num_cpus::get()).max(1);
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

    pub fn push_job(&self, job_task: Arc<RwLock<JobTask + Send + Sync + 'static>>) -> Arc<JobCounter> {
        self.thread_wake_event.wake_threads(); // notify threads to wake
        let job_counter = Arc::new(JobCounter::new(1));
        let job_descriptor = JobDescriptor::new(job_task, job_counter.clone());
        self.job_queue.push(job_descriptor);
        job_counter
    }

    pub fn push_job_array(&self, job_tasks: &Vec<Arc<RwLock<JobTask + Send + Sync + 'static>>>) -> Arc<JobCounter> {
        self.thread_wake_event.wake_threads(); // notify threads to wake
        let job_counter = Arc::new(JobCounter::new(job_tasks.len()));
        for job in job_tasks {
            let job_descriptor = JobDescriptor::new(job.clone(), job_counter.clone());
            self.job_queue.push(job_descriptor);
        }
        job_counter
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
    job: Arc<RwLock<JobTask + Send + Sync + 'static>>,
    job_counter: Arc<JobCounter>,
}

impl JobDescriptor {
    pub fn new(job: Arc<RwLock<JobTask + Send + Sync + 'static>>, job_counter: Arc<JobCounter>) -> JobDescriptor {
        JobDescriptor {
            job: job,
            job_counter,
        }
    }

    fn run(&self) {
        self.job.write().run();
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
        let mut queue = self.queue.write();
        queue.push_back(descriptor);
    }

    fn pop(&self) -> Option<JobDescriptor> {
        let mut queue = self.queue.write();
        queue.pop_front()
    }

    fn is_empty(&self) -> bool {
        let queue = self.queue.read();
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
        *self.is_running.write() = false;
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
        let mut wake = lock.lock();
        *wake = true;
        condvar.notify_all();
    }

    fn sleep_thread(&self) {
        let &(ref lock, ref condvar) = &*self.value;
        // sleep on event, this may wake spuriously but we don't really care
        condvar.wait(&mut lock.lock());
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
        while *self.is_running.read() {
            match self.queue.pop() {
                Some(job_descriptor) => {
                    job_descriptor.run();
                    job_descriptor.job_counter.decrement();
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

#[derive(Clone)]
pub struct MultiSliceReadWriteLock<T> {

    data: Arc<UnsafeCell<T>>
}

unsafe impl<T> Send for MultiSliceReadWriteLock<T> {}
unsafe impl<T> Sync for MultiSliceReadWriteLock<T> {}

impl<T> MultiSliceReadWriteLock<T> {
    
    pub fn new(data: T) -> MultiSliceReadWriteLock<T> {
        MultiSliceReadWriteLock {
            data: Arc::new(UnsafeCell::new(data))
        }    
    }
    
    pub fn write(&self) -> &mut T {
        // TODO(SS): Ensure no one else can grab reference to same slice twice
        unsafe {  &mut *self.data.get() }
    }
    
    pub fn read(&self) -> &T {
        // TODO(SS): Ensure no one can read when write is checked out?
        unsafe {  & *self.data.get() }
    }
}