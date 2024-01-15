use crate::db::db_trait::Db;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct ProjectInfoArgs {
    /// Project name
    name: String,
}

impl ProjectInfoArgs {
    pub async fn show_info(&self, db_handler: &dyn Db) -> anyhow::Result<()> {
        let project = db_handler.get_project(&self.name).await?;
        println!("{}\n", project);

        println!("{}", "Actions:".to_string().red().underline());
        let actions = db_handler.get_actions(Some(&self.name)).await?;
        actions
            .iter()
            .enumerate()
            .for_each(|(i, action)| println!("   {}. {}", i + 1, action));
        Ok(())
    }
}
