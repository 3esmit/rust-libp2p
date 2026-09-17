use libp2p_identity::Keypair;

use super::super::{select_muxer::SelectMuxerUpgrade, select_security::SelectSecurityUpgrade};

#[allow(unreachable_pub)]
pub trait IntoSecurityUpgrade<C> {
    type Upgrade;
    type Error;

    fn into_security_upgrade(self, keypair: &Keypair) -> Result<Self::Upgrade, Self::Error>;
}

impl<C, T, F, E> IntoSecurityUpgrade<C> for F
where
    F: for<'a> FnOnce(&'a Keypair) -> Result<T, E>,
{
    type Upgrade = T;
    type Error = E;

    fn into_security_upgrade(self, keypair: &Keypair) -> Result<Self::Upgrade, Self::Error> {
        (self)(keypair)
    }
}

impl<F1, F2, C> IntoSecurityUpgrade<C> for (F1, F2)
where
    F1: IntoSecurityUpgrade<C>,
    F2: IntoSecurityUpgrade<C>,
{
    type Upgrade = SelectSecurityUpgrade<F1::Upgrade, F2::Upgrade>;
    type Error = either::Either<F1::Error, F2::Error>;

    fn into_security_upgrade(self, keypair: &Keypair) -> Result<Self::Upgrade, Self::Error> {
        let (f1, f2) = self;

        let u1 = f1
            .into_security_upgrade(keypair)
            .map_err(either::Either::Left)?;
        let u2 = f2
            .into_security_upgrade(keypair)
            .map_err(either::Either::Right)?;

        Ok(SelectSecurityUpgrade::new(u1, u2))
    }
}

#[allow(unreachable_pub)]
pub trait IntoMultiplexerUpgrade<C> {
    type Upgrade;

    fn into_multiplexer_upgrade(self) -> Self::Upgrade;
}

impl<C, U, F> IntoMultiplexerUpgrade<C> for F
where
    F: FnOnce() -> U,
{
    type Upgrade = U;

    fn into_multiplexer_upgrade(self) -> Self::Upgrade {
        (self)()
    }
}

impl<C, U1, U2> IntoMultiplexerUpgrade<C> for (U1, U2)
where
    U1: IntoMultiplexerUpgrade<C>,
    U2: IntoMultiplexerUpgrade<C>,
{
    type Upgrade = SelectMuxerUpgrade<U1::Upgrade, U2::Upgrade>;

    fn into_multiplexer_upgrade(self) -> Self::Upgrade {
        let (f1, f2) = self;

        let u1 = f1.into_multiplexer_upgrade();
        let u2 = f2.into_multiplexer_upgrade();

        SelectMuxerUpgrade::new(u1, u2)
    }
}
