use std::{
    sync::{mpsc, Arc, Mutex}, 
    thread,
    collections::HashMap
};

// Alias for a job, which is a boxed function that satisfies certain trait bounds
type Job = Box<dyn FnOnce() + Send + 'static>;

// Struct representing the load balancer
pub struct LoadBalancer {
    workers: Vec<Worker>,  // Vector of worker threads
    sender: mpsc::Sender<BalancerMessage>,  // Channel sender to dispatch jobs
    load: Arc<Mutex<HashMap<usize, usize>>>,  // Map to track the number of active jobs per worker
}

// Enum for messages that can be sent to the load balancer
enum BalancerMessage {
    NewJob(Job),  // Message type for new job
    JobFinished(usize),  // Message type for finished job with worker ID
}

// Implementation of the LoadBalancer
impl LoadBalancer {
    // Constructor method to create a new LoadBalancer
    pub fn new(size: usize) -> LoadBalancer {
        assert!(size > 0);  // Ensure the size is greater than 0

        let (sender, receiver) = mpsc::channel();  // Create a channel for communication
        let receiver = Arc::new(Mutex::new(receiver));  // Wrap receiver in Arc and Mutex for thread-safe shared ownership
        let load = Arc::new(Mutex::new(HashMap::new()));  // Initialize the load map

        let mut workers = Vec::with_capacity(size);  // Create a vector to hold workers

        // Spawn worker threads
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver), Arc::clone(&load), sender.clone()));
            load.lock().unwrap().insert(id, 0);  // Initialize each worker's load to 0
        }

        LoadBalancer { workers, sender, load }
    }

    // Method to add a new job to the load balancer
    pub fn execute<F>(&self, f: F)
    where 
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);  // Wrap the job in a Box
        self.sender.send(BalancerMessage::NewJob(job)).unwrap();  // Send a NewJob message
    }

    // Private method to update the load when a job is finished by a worker
    fn worker_did_finish_job(&self, worker_id: usize) {
        let mut load = self.load.lock().unwrap();
        if let Some(count) = load.get_mut(&worker_id) {
            *count -= 1;  // Decrement the job count for the worker
        }
    }
}

// Implementation of the Drop trait for LoadBalancer
impl Drop for LoadBalancer {
    fn drop(&mut self) {
        // Ensure all worker threads are properly joined when the LoadBalancer is dropped
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

// Struct representing a worker thread
struct Worker {
    id: usize,  // ID of the worker
    thread: Option<thread::JoinHandle<()>>,  // Handle to the thread
}

// Implementation of the Worker
impl Worker {
    // Constructor method to create a new Worker
    fn new(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<BalancerMessage>>>,
        load: Arc<Mutex<HashMap<usize, usize>>>,
        balancer_sender: mpsc::Sender<BalancerMessage>
    ) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();  // Wait for a message from the load balancer

            match message {
                BalancerMessage::NewJob(job) => {
                    {
                        let mut load = load.lock().unwrap();
                        if let Some(count) = load.get_mut(&id) {
                            *count += 1;  // Increment the job count for this worker
                        }
                    }
                    println!("Worker {id} got a job; executing.");
                    job();  // Execute the job
                    // Notify the load balancer that the job is finished
                    balancer_sender.send(BalancerMessage::JobFinished(id)).unwrap();
                },
                BalancerMessage::JobFinished(_) => { /* Ignore this message in the worker */ }
            }
        });
        Worker { id, thread: Some(thread) }
    }
}
