use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::collections::VecDeque;

pub struct Jobs {
    thread_pool: ThreadPool,
}

impl Jobs {
    pub fn new() -> Jobs {
        Jobs {
            thread_pool: ThreadPool::new()
        }
    }

    pub fn run(&self, job_descriptor: JobDescriptor) {
        self.thread_pool.job_queue.push(job_descriptor);
    }

    pub fn wait_for_jobs(&self) {
        // TODO(SS) insert job and wait for completion
    }
}

#[allow(dead_code)]
struct ThreadPool {
    job_threads: Vec<JobThreadHandle>,
    job_queue: JobQueue,
}

#[allow(dead_code)]
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
    // TODO(SS): Call on drop 
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

pub struct JobDescriptor {
    pub value: String,
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

    fn pop(&self) -> Option<JobDescriptor> {
        let mut queue = self.queue.write().unwrap();
        queue.pop_front()
    }

    // low contention - will return false if queue is already locked for write
    fn is_empty(&self) -> bool {
        return match self.queue.try_read() {
            Ok(queue) => queue.is_empty(),
            Err(_) => false,
        };
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
                    println!("thread {}: Running job {}", self.thread_pool_index, job_descriptor.value );
                },
                None => {
                    // TODO(SS): Wake on event
                    thread::sleep(Duration::from_secs(1))
                },
            };
        }
        
        println!("Job Thread: {} stopped..", self.thread_pool_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {

        let jobs = Jobs::new();

        for i in 0..40 {

            jobs.run(JobDescriptor{value: format!("job {}", i)});
        }

        // TODO(SS): thread_pool.wait_for_jobs();
        thread::sleep(Duration::from_secs(10));
    }
}
