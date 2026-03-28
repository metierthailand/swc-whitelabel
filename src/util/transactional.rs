use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

thread_local! {
  static TX_FS: RefCell<TxFS> = RefCell::new(TxFS::default())
}

#[derive(Default)]
pub struct TxFS {
    /// Holds the pending writes: (Destination Path, New File Contents)
    pending_writes: Vec<(PathBuf, Vec<u8>)>,
}

impl TxFS {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_buffer<F, R>(f: F) -> R
    where
        F: FnOnce(&mut TxFS) -> R,
    {
        TX_FS.with(|tx| {
            // borrow_mut() enforces that you only have one mutable
            // reference active at a time on this thread!
            let mut tx_fs = tx.borrow_mut();
            f(&mut tx_fs)
        })
    }

    /// Buffers a file write in memory. Does not touch the filesystem.
    /// Note: Added `&mut self` so it can store the state!
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &mut self,
        path: P,
        contents: C,
    ) -> io::Result<()> {
        self.pending_writes
            .push((path.as_ref().to_path_buf(), contents.as_ref().to_vec()));
        Ok(())
    }

    /// Attempts to write all buffered files to disk.
    /// If any write fails, it rolls back all previously modified files in this transaction.
    pub fn commit(&mut self) -> io::Result<()> {
        // 1. Snapshot the current state of files we are about to overwrite
        let mut backups: Vec<(PathBuf, Vec<u8>)> = Vec::new();

        for (path, _) in &self.pending_writes {
            if path.exists() {
                match fs::read(path) {
                    Ok(original_content) => backups.push((path.clone(), original_content)),
                    Err(e) => {
                        // If we can't even read the file to back it up, abort the whole transaction
                        return Err(io::Error::other(format!(
                            "Transaction aborted: Failed to backup {}: {}",
                            path.display(),
                            e
                        )));
                    }
                }
            }
        }
        let writes = std::mem::take(&mut self.pending_writes);
        // 2. Execute the writes
        for (path, new_content) in writes {
            if let Err(write_err) = fs::write(&path, new_content) {
                // 3. 🚨 ROLLBACK TRIGGERED 🚨
                // A write failed! Restore all the files we modified so far.
                for (backup_path, original_content) in backups {
                    // We make a best-effort attempt to restore.
                    let _ = fs::write(&backup_path, original_content);
                }

                self.pending_writes.clear();
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    format!(
                        "Transaction failed on {:?}. Rolled back previous writes. Error: {}",
                        path.display(),
                        write_err
                    ),
                ));
            }
        }

        // 4. Success! Clear the buffer for the next transaction.
        self.pending_writes.clear();
        Ok(())
    }
}
