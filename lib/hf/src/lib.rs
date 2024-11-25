use anyhow::Result;
use hf_hub::{
    api::sync::{Api, ApiRepo},
    Repo, RepoType,
};
use parquet::file::reader::SerializedFileReader;
use std::fs::File;

fn sibling_to_parquet(rfilename: &str, repo: &ApiRepo) -> Result<SerializedFileReader<File>> {
    let local = repo.get(rfilename)?;
    let file = File::open(local)?;
    let reader = SerializedFileReader::new(file)?;
    Ok(reader)
}

pub fn from_hub(api: &Api, dataset_id: String) -> Result<Vec<SerializedFileReader<File>>> {
    let repo = Repo::with_revision(
        dataset_id,
        RepoType::Dataset,
        "refs/convert/parquet".to_string(),
    );
    let repo = api.repo(repo);
    let info = repo.info()?;

    let files: Result<Vec<_>, _> = info
        .siblings
        .into_iter()
        .filter_map(|s| -> Option<Result<_, _>> {
            let filename = s.rfilename;
            if filename.ends_with(".parquet") {
                let reader_result = sibling_to_parquet(&filename, &repo);
                Some(reader_result)
            } else {
                None
            }
        })
        .collect();
    let files = files?;

    Ok(files)
}
