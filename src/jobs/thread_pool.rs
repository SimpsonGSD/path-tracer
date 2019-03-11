use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::sync::{Arc, RwLock, Mutex};

pub struct ThreadPool {
    job_threads: Vec<JobThreadHandle>,
}

impl ThreadPool {
    pub fn new() -> ThreadPool {
        let num_cores = num_cpus::get();
        println!("Thread pool: Spooling up {} threads", num_cores);
        let mut job_threads = vec![];
        for i in 0..num_cores {
            let job_thread = JobThread::new(i);
            job_threads.push(job_thread);
        }

        ThreadPool {
            job_threads
        }
    }

    pub fn destroy(mut self) {
        // stop each thread before waiting for them all to join
        self.job_threads.iter().for_each(|thread| thread.stop());
        // drain all threads and wait for them to join
        self.job_threads.drain(..).for_each( move |thread| thread.join());
    }
}

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
    is_running: Arc<RwLock<bool>>
}

impl JobThread {
    fn new(thread_pool_index: usize) -> JobThreadHandle {
        let is_running = Arc::new(RwLock::new(true));
        let job_thread = JobThread {
            thread_pool_index,
            is_running: is_running.clone()
        };

        let thread_handle = thread::spawn( move || {
            job_thread.run();
        });

        JobThreadHandle {
            is_running, 
            thread_handle
        }
    }

    fn run(&self) {
        println!("Job Thread {} started..", self.thread_pool_index);
        
        while *self.is_running.read().unwrap() {
            thread::sleep(Duration::from_secs(1));
            println!("Job Thread {} running", self.thread_pool_index);
        }
        
        println!("Job Thread: {} stopped..", self.thread_pool_index);
    }
}
