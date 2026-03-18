pub mod system_transfer;
pub mod token_transfer;
pub mod swap_detect;
pub mod mint_burn;
pub mod stake_ops;
pub mod deploy_detect;
pub mod nft_ops;
pub mod compute_budget;

pub use system_transfer::SystemTransferModule;
pub use token_transfer::TokenTransferModule;
pub use swap_detect::SwapDetectModule;
pub use mint_burn::MintBurnModule;
pub use stake_ops::StakeOpsModule;
pub use deploy_detect::DeployDetectModule;
pub use nft_ops::NftOpsModule;
pub use compute_budget::ComputeBudgetModule;
