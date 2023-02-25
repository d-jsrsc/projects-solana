// sell nft with token back (only support WenCoin, USDT, USDC)

#[account] // nft 2 token
pub struct MarketNftToTokenAccount {
    pub version: u32,
    pub creator: Pubkey,
    pub nft_token: Pubkey, // 质押的 NFT
    pub nft_amount: u64,   //
    pub token: Pubkey,     // 期待换回的物品
    pub amount: u64,       // 期待换回的数量
    pub create_time: i64,
}
