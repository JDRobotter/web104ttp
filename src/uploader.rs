use anyhow::{Result, Error};

use rand::Rng;
use std::collections::HashMap;

struct Session {
    picture_id: u32,
    picture_side: u32,
    
    _total_size: usize,
    bytes: Vec<u8>,
}

impl Session {
    fn new(picture_id: u32, picture_side: u32, total_size: usize) -> Self {
        
        // allocate a null vector of the requested size

        Self {
            picture_id,
            picture_side,
            _total_size:total_size,
            bytes: vec![0; total_size],
        }
    }
}

pub struct FileUploader {
    sessions: HashMap<u32, Session>,
}

impl FileUploader {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn generate_uid(&self) -> Result<u32> {
        loop {
            // generate a new u32 ID and check if it already exists in sessions
            let uid:u32 = rand::thread_rng().gen();
            if !self.sessions.contains_key(&uid) {
                return Ok(uid)
            }
        }
    }

    pub fn clean_sessions(&mut self) {
        // sessions
    }

    pub fn new_session(&mut self, n: u32, side: u32, size:usize) -> Result<u32> {

        let uid = self.generate_uid()?;

        self.sessions.insert(uid, Session::new(n, side, size));

        Ok(uid)
    }

    pub fn add_chunk(&mut self, uid: u32, position:usize, chunk: &[u8]) -> Result<()> {

        // find upload session
        let session = self.sessions.get_mut(&uid)
            .ok_or(Error::msg("no session"))?;

        let a = position;
        let b = position + chunk.len();
        session.bytes[a..b].clone_from_slice(chunk);

        Ok(())
    }

    pub fn take(&mut self, uid: u32) -> Result<(u32, u32, Vec<u8>)> {

        let session = self.sessions.remove(&uid)
            .ok_or(Error::msg("no session"))?;
        
        Ok((session.picture_id, session.picture_side, session.bytes))
    }
}
