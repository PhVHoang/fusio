mod fs;

use std::{ops::Range, sync::Arc};

use object_store::{aws::AmazonS3, path::Path, GetOptions, GetRange, ObjectStore, PutPayload};

use crate::{buf::IoBufMut, Error, IoBuf, Read, Seek, Write};

pub struct S3File {
    inner: Arc<AmazonS3>,
    path: Path,
    pos: u64,
}

impl Read for S3File {
    async fn read_exact<B: IoBufMut>(&mut self, mut buf: B) -> Result<B, Error> {
        let pos = self.pos as usize;

        let mut opts = GetOptions::default();
        let range = GetRange::Bounded(Range {
            start: pos,
            end: pos + buf.bytes_init(),
        });
        opts.range = Some(range);

        let result = self.inner.get_opts(&self.path, opts).await?;
        let bytes = result.bytes().await?;

        self.pos += bytes.len() as u64;

        buf.as_slice_mut().copy_from_slice(&bytes);
        Ok(buf)
    }

    async fn size(&self) -> Result<u64, Error> {
        let options = GetOptions {
            head: true,
            ..Default::default()
        };
        let response = self.inner.get_opts(&self.path, options).await?;
        Ok(response.meta.size as u64)
    }
}

impl Seek for S3File {
    async fn seek(&mut self, pos: u64) -> Result<(), Error> {
        self.pos = pos;
        Ok(())
    }
}

impl Write for S3File {
    async fn write_all<B: IoBuf>(&mut self, buf: B) -> (Result<(), Error>, B) {
        let result = self
            .inner
            .put(&self.path, PutPayload::from_bytes(buf.as_bytes()))
            .await
            .map(|_| ())
            .map_err(Error::ObjectStore);

        (result, buf)
    }

    async fn sync_data(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn sync_all(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[cfg(all(feature = "tokio", not(feature = "completion-based")))]
    #[tokio::test]
    async fn test_s3() {
        use std::{env, env::VarError, sync::Arc};

        use bytes::Bytes;
        use object_store::{aws::AmazonS3Builder, ObjectStore};

        use crate::{remotes::object_store::S3File, Read, Write};

        let fn_env = || {
            let region = env::var("TEST_INTEGRATION")?;
            let bucket_name = env::var("TEST_INTEGRATION")?;
            let access_key_id = env::var("TEST_INTEGRATION")?;
            let secret_access_key = env::var("TEST_INTEGRATION")?;

            Ok::<(String, String, String, String), VarError>((
                region,
                bucket_name,
                access_key_id,
                secret_access_key,
            ))
        };
        if let Ok((region, bucket_name, access_key_id, secret_access_key)) = fn_env() {
            let path = object_store::path::Path::parse("/test_file").unwrap();
            let s3 = AmazonS3Builder::new()
                .with_region(region)
                .with_bucket_name(bucket_name)
                .with_access_key_id(access_key_id)
                .with_secret_access_key(secret_access_key)
                .build()
                .unwrap();
            let _ = s3.delete(&path).await;

            let mut store = S3File {
                inner: Arc::new(s3),
                path,
                pos: 0,
            };
            let (result, bytes) = store.write_all(Bytes::from("hello! Fusio!")).await;
            result.unwrap();

            let buf = vec![0_u8; bytes.len()];
            let buf = store.read_exact(&mut buf[..]).await.unwrap();
            assert_eq!(buf, &bytes[..]);
        }
    }
}
