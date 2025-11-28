use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
pub enum ProcState { Run, Sleep, Stop, Zombie }

pub struct Process { pub pid: u32, pub ppid: u32, pub name: String, pub state: ProcState }

pub struct ProcessTable { next_pid: u32, procs: HashMap<u32, Process> }
impl ProcessTable { pub fn new() -> Self { ProcessTable { next_pid: 1, procs: HashMap::new() } } pub fn spawn(&mut self, name:&str, ppid:u32)->u32 { let pid = self.next_pid; self.next_pid+=1; self.procs.insert(pid, Process{pid,ppid,name:name.into(),state:ProcState::Run}); pid } pub fn list(&self)->Vec<&Process>{ let mut v:Vec<_>=self.procs.values().collect(); v.sort_by_key(|p| p.pid); v } pub fn kill(&mut self,pid:u32)->bool { if pid<=1 {return false;} self.procs.remove(&pid).is_some() } }

pub struct Scheduler { run_queue: Vec<u32>, cursor: usize }
impl Scheduler { pub fn new()->Self { Scheduler{ run_queue:Vec::new(), cursor:0 } } pub fn add(&mut self,pid:u32){ if !self.run_queue.contains(&pid){ self.run_queue.push(pid);} } pub fn tick(&mut self){ if self.run_queue.is_empty(){return;} self.cursor=(self.cursor+1)%self.run_queue.len(); } pub fn current(&self)->Option<u32>{ self.run_queue.get(self.cursor).cloned() } }
