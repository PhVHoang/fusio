use std::io;

use async_stream::stream;
use futures_core::Stream;
use tokio::{
    fs::{create_dir_all, remove_file, File},
    task::spawn_blocking,
};

use crate::{
    fs::{FileMeta, Fs, OpenOptions, WriteMode},
    path::{path_to_local, Path},
    Error,
};

pub struct TokioFs;

impl Fs for TokioFs {
    type File = File;

    async fn open_options(&self, path: &Path, options: OpenOptions) -> Result<Self::File, Error> {
        let local_path = path_to_local(path)?;

        Ok(tokio::fs::OpenOptions::new()
            .read(options.read)
            .write(options.write.is_some())
            .create(options.create)
            .append(options.write == Some(WriteMode::Append))
            .truncate(options.write == Some(WriteMode::Truncate))
            .open(&local_path)
            .await?)
    }

    async fn create_dir_all(path: &Path) -> Result<(), Error> {
        let path = path_to_local(path)?;
        create_dir_all(path).await?;

        Ok(())
    }

    async fn list(
        &self,
        path: &Path,
    ) -> Result<impl Stream<Item = Result<FileMeta, Error>>, Error> {
        let path = path_to_local(path)?;

        spawn_blocking(move || {
            let entries = path.read_dir()?;
            Ok::<_, Error>(stream! {
                for entry in entries {
                    let entry = entry?;
                    yield Ok(FileMeta { path: Path::from_filesystem_path(entry.path())?, size: entry.metadata()?.len() });
                }
            })
        })
        .await
        .map_err(io::Error::from)?
    }

    async fn remove(&self, path: &Path) -> Result<(), Error> {
        let path = path_to_local(path)?;

        remove_file(&path).await?;
        Ok(())
    }
}
