use crate::*;

static ASSET_FARMS: Lazy<Mutex<HashMap<FarmId, Option<AssetFarm>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const NANOS_PER_DAY: Duration = 24 * 60 * 60 * 10u64.pow(9);

/// A data required to keep track of a farm for an account.
#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetFarm {
    #[serde(with = "u64_dec_format")]
    pub block_timestamp: Timestamp,
    /// Active rewards for the farm
    pub rewards: HashMap<TokenId, AssetFarmReward>,
    /// Inactive rewards
    #[serde(skip_serializing)]
    pub inactive_rewards: LookupMap<TokenId, VAssetFarmReward>,
}

impl Clone for AssetFarm {
    fn clone(&self) -> Self {
        Self {
            block_timestamp: self.block_timestamp,
            rewards: self.rewards.clone(),
            inactive_rewards: BorshDeserialize::try_from_slice(
                &self.inactive_rewards.try_to_vec().unwrap(),
            )
            .unwrap(),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum VAssetFarmReward {
    Current(AssetFarmReward),
}

impl From<VAssetFarmReward> for AssetFarmReward {
    fn from(v: VAssetFarmReward) -> Self {
        match v {
            VAssetFarmReward::Current(c) => c,
        }
    }
}

impl From<AssetFarmReward> for VAssetFarmReward {
    fn from(c: AssetFarmReward) -> Self {
        VAssetFarmReward::Current(c)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Serialize, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetFarmReward {
    /// The amount of reward distributed per day.
    #[serde(with = "u128_dec_format")]
    pub reward_per_day: Balance,
    /// The log base for the booster. Used to compute boosted shares per account.
    /// Including decimals of the booster.
    #[serde(with = "u128_dec_format")]
    pub booster_log_base: Balance,

    /// The amount of rewards remaining to distribute.
    #[serde(with = "u128_dec_format")]
    pub remaining_rewards: Balance,

    /// The total number of boosted shares.
    #[serde(with = "u128_dec_format")]
    pub boosted_shares: Balance,
    #[serde(skip)]
    pub reward_per_share: BigDecimal,
}

impl AssetFarm {
    pub fn update(&mut self) {
        let block_timestamp = env::block_timestamp();
        if block_timestamp == self.block_timestamp {
            return;
        }
        let time_diff = block_timestamp - self.block_timestamp;
        self.block_timestamp = block_timestamp;
        let mut new_inactive_reward = vec![];
        for (token_id, reward) in self.rewards.iter_mut() {
            if reward.boosted_shares == 0 {
                continue;
            }
            let acquired_rewards = std::cmp::min(
                reward.remaining_rewards,
                u128_ratio(
                    reward.reward_per_day,
                    u128::from(time_diff),
                    u128::from(NANOS_PER_DAY),
                ),
            );
            reward.remaining_rewards -= acquired_rewards;
            reward.reward_per_share = reward.reward_per_share
                + BigDecimal::from(acquired_rewards) / BigDecimal::from(reward.boosted_shares);
            if reward.remaining_rewards == 0 {
                new_inactive_reward.push(token_id.clone());
            }
        }
        for token_id in new_inactive_reward {
            let reward = self.rewards.remove(&token_id).unwrap();
            self.internal_set_inactive_asset_farm_reward(&token_id, reward);
        }
    }

    pub fn internal_get_inactive_asset_farm_reward(
        &self,
        token_id: &TokenId,
    ) -> Option<AssetFarmReward> {
        self.inactive_rewards.get(token_id).map(|o| o.into())
    }

    pub fn internal_remove_inactive_asset_farm_reward(
        &mut self,
        token_id: &TokenId,
    ) -> Option<AssetFarmReward> {
        self.inactive_rewards.remove(token_id).map(|o| o.into())
    }

    pub fn internal_set_inactive_asset_farm_reward(
        &mut self,
        token_id: &TokenId,
        asset_farm_reward: AssetFarmReward,
    ) {
        self.inactive_rewards
            .insert(token_id, &asset_farm_reward.into());
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum VAssetFarm {
    Current(AssetFarm),
}

impl From<VAssetFarm> for AssetFarm {
    fn from(v: VAssetFarm) -> Self {
        match v {
            VAssetFarm::Current(c) => c,
        }
    }
}

impl From<AssetFarm> for VAssetFarm {
    fn from(c: AssetFarm) -> Self {
        VAssetFarm::Current(c)
    }
}

impl Contract {
    pub fn internal_unwrap_asset_farm(&self, farm_id: &FarmId) -> AssetFarm {
        self.internal_get_asset_farm(farm_id)
            .expect("Asset farm not found")
    }

    pub fn internal_get_asset_farm(&self, farm_id: &FarmId) -> Option<AssetFarm> {
        let mut cache = ASSET_FARMS.lock().unwrap();
        cache.get(farm_id).cloned().unwrap_or_else(|| {
            let asset_farm = self.asset_farms.get(farm_id).map(|v| {
                let mut asset_farm: AssetFarm = v.into();
                asset_farm.update();
                asset_farm
            });
            cache.insert(farm_id.clone(), asset_farm.clone());
            asset_farm
        })
    }

    pub fn internal_set_asset_farm(&mut self, farm_id: &FarmId, asset_farm: AssetFarm) {
        ASSET_FARMS
            .lock()
            .unwrap()
            .insert(farm_id.clone(), Some(asset_farm.clone()));
        self.asset_farms.insert(farm_id, &asset_farm.into());
    }
}

#[near_bindgen]
impl Contract {
    /// Returns an asset farm for a given farm ID.
    pub fn get_asset_farm(&self, farm_id: FarmId) -> Option<AssetFarm> {
        self.internal_get_asset_farm(&farm_id)
    }

    /// Returns a list of pairs (farm ID, asset farm) for a given list of farm IDs.
    pub fn get_asset_farms(&self, farm_ids: Vec<FarmId>) -> Vec<(FarmId, AssetFarm)> {
        farm_ids
            .into_iter()
            .filter_map(|farm_id| {
                self.internal_get_asset_farm(&farm_id)
                    .map(|asset_farm| (farm_id, asset_farm))
            })
            .collect()
    }

    /// Returns a list of pairs (farm ID, asset farm) from a given index up to a given limit.
    ///
    /// Note, the number of returned elements may be twice larger than the limit, due to the
    /// pagination implementation. To continue to the next page use `from_index + limit`.
    pub fn get_asset_farms_paged(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<(FarmId, AssetFarm)> {
        let keys = self.asset_ids.as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());
        let mut farm_ids = vec![];
        for index in from_index..std::cmp::min(keys.len(), limit) {
            let token_id = keys.get(index).unwrap();
            farm_ids.push(FarmId::Supplied(token_id.clone()));
            farm_ids.push(FarmId::Borrowed(token_id));
        }
        self.get_asset_farms(farm_ids)
    }
}
