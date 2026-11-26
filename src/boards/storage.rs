use directories::BaseDirs;
use rusqlite::{Connection, OptionalExtension};
use std::{error::Error, fs, io, time::Duration};

use super::Boards;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct Store(Connection);

impl Store {
    pub fn open() -> Result<Self> {
        let base = BaseDirs::new().ok_or_else(|| io::Error::other("missing data directory"))?;
        let path = base.data_local_dir().join("dev.edfloreshz.Boards");
        fs::create_dir_all(&path)?;
        Self::initialize(Connection::open(path.join("boards.sqlite3"))?)
    }

    fn initialize(mut connection: Connection) -> Result<Self> {
        connection.busy_timeout(Duration::from_millis(250))?;
        let transaction = connection.transaction()?;
        let version: u32 =
            transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
        match version {
            0 => transaction.execute_batch(
                "CREATE TABLE workspace (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    document TEXT NOT NULL
                );
                PRAGMA user_version = 1;",
            )?,
            1 => {}
            _ => {
                return Err(
                    io::Error::other(format!("unsupported database version {version}")).into(),
                );
            }
        }
        transaction.commit()?;
        Ok(Self(connection))
    }

    pub fn load(&self) -> Result<Boards> {
        let document: Option<String> = self
            .0
            .query_row("SELECT document FROM workspace WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        document.map_or_else(
            || Ok(Boards::default()),
            |data| Ok(serde_json::from_str(&data)?),
        )
    }

    pub fn save(&self, boards: &Boards) -> Result<()> {
        self.0.execute(
            "INSERT INTO workspace (id, document) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET document = excluded.document",
            [serde_json::to_string(boards)?],
        )?;
        Ok(())
    }
}
