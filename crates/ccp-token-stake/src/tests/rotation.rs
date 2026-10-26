use crate::tests::StakedChainAlg;
use alux_ext::ext;
use anyhow::Result;
use ccp_stake::StakeRotationLawsExt;

/// States what validator rotation means on every chain, whatever moves its stake.
#[ext(name = RotationScenariosExt)]
pub(crate) impl<This> This
where
    This: StakedChainAlg,
{
    /// Runs four epochs in which `change` moves stake, then checks that every epoch's weights were
    /// settled before it began and that rotation between any two epochs is their difference.
    fn scenario_rotation(&mut self, change: impl Fn(&mut Self, u64)) -> Result<()> {
        self.scenario_epoch_close();
        for epoch in 1..5 {
            self.check_weights_settled(|this| change(this, epoch))?;
            self.scenario_epoch_close();
        }
        for first in 0..5 {
            self.check_rotation_identity(&first)?;
            for second in 0..5 {
                self.check_rotation_reconstructs(&first, &second)?;
                self.check_rotation_inverse(&first, &second)?;
                (0..5).try_for_each(|third| self.check_turnover_distance(&first, &second, &third))?;
            }
        }
        Ok(())
    }
}
