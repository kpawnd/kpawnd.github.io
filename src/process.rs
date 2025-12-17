use std::collections::{HashMap, VecDeque};

#[derive(Clone, Copy, PartialEq)]
pub enum ProcState {
    Run,
    Sleep,
    Stop,
    Zombie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    High = 3,
    Normal = 2,
    Low = 1,
}

pub struct Process {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub state: ProcState,
    pub priority: Priority,
    pub time_slice: u32,
    pub remaining_slice: u32,
    pub memory_offset: u32,    // Memory block offset allocated for this process
    pub memory_size: u32,      // Size of memory allocated for this process
}

pub struct ProcessTable {
    next_pid: u32,
    procs: HashMap<u32, Process>,
}
impl Default for ProcessTable {
    fn default() -> Self {
        Self::new()
    }
}
impl ProcessTable {
    pub fn new() -> Self {
        ProcessTable {
            next_pid: 1,
            procs: HashMap::new(),
        }
    }
    pub fn spawn(&mut self, name: &str, ppid: u32, memory: &mut crate::memory::Memory) -> Option<u32> {
        self.spawn_with_priority(name, ppid, Priority::Normal, memory)
    }

    pub fn spawn_with_priority(&mut self, name: &str, ppid: u32, priority: Priority, memory: &mut crate::memory::Memory) -> Option<u32> {
        // Allocate memory for the process (stack + heap)
        let process_memory_size = match priority {
            Priority::High => 128 * 1024,    // 128KB for high priority
            Priority::Normal => 64 * 1024,   // 64KB for normal priority  
            Priority::Low => 32 * 1024,      // 32KB for low priority
        };

        let memory_offset = match memory.alloc(process_memory_size) {
            Some(offset) => offset,
            None => return None, // Out of memory
        };

        let pid = self.next_pid;
        self.next_pid += 1;

        let time_slice = match priority {
            Priority::High => 150,
            Priority::Normal => 100,
            Priority::Low => 50,
        };

        self.procs.insert(
            pid,
            Process {
                pid,
                ppid,
                name: name.into(),
                state: ProcState::Run,
                priority,
                time_slice,
                remaining_slice: time_slice,
                memory_offset,
                memory_size: process_memory_size,
            },
        );
        Some(pid)
    }

    pub fn list(&self) -> Vec<&Process> {
        let mut v: Vec<_> = self.procs.values().collect();
        v.sort_by_key(|p| (std::cmp::Reverse(p.priority), p.pid));
        v
    }

    pub fn kill(&mut self, pid: u32, memory: &mut crate::memory::Memory) -> bool {
        if pid <= 1 {
            return false;
        }
        if let Some(process) = self.procs.remove(&pid) {
            // Free the process memory
            memory.free(process.memory_offset);
            true
        } else {
            false
        }
    }

    pub fn get_mut(&mut self, pid: u32) -> Option<&mut Process> {
        self.procs.get_mut(&pid)
    }
}

pub struct Scheduler {
    high_queue: VecDeque<u32>,
    normal_queue: VecDeque<u32>,
    low_queue: VecDeque<u32>,
    current: Option<u32>,
}
impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            high_queue: VecDeque::new(),
            normal_queue: VecDeque::new(),
            low_queue: VecDeque::new(),
            current: None,
        }
    }

    pub fn add(&mut self, pid: u32, priority: Priority) {
        match priority {
            Priority::High => {
                if !self.high_queue.contains(&pid) {
                    self.high_queue.push_back(pid);
                }
            }
            Priority::Normal => {
                if !self.normal_queue.contains(&pid) {
                    self.normal_queue.push_back(pid);
                }
            }
            Priority::Low => {
                if !self.low_queue.contains(&pid) {
                    self.low_queue.push_back(pid);
                }
            }
        }
    }

    pub fn tick(&mut self, process_table: &mut ProcessTable) {
        // Check if current process exhausted time slice
        if let Some(pid) = self.current {
            if let Some(process) = process_table.get_mut(pid) {
                process.remaining_slice = process.remaining_slice.saturating_sub(1);

                if process.remaining_slice == 0 {
                    // Reset time slice and move to back of queue
                    process.remaining_slice = process.time_slice;
                    self.add(pid, process.priority);
                    self.current = None;
                }
            }
        }

        // If no current process, select next from highest priority queue
        if self.current.is_none() {
            self.current = self
                .high_queue
                .pop_front()
                .or_else(|| self.normal_queue.pop_front())
                .or_else(|| self.low_queue.pop_front());
        }
    }

    pub fn current(&self) -> Option<u32> {
        self.current
    }

    pub fn remove(&mut self, pid: u32) {
        self.high_queue.retain(|&p| p != pid);
        self.normal_queue.retain(|&p| p != pid);
        self.low_queue.retain(|&p| p != pid);
        if self.current == Some(pid) {
            self.current = None;
        }
    }
}
