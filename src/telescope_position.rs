#[derive(Debug, Clone, Copy)]
pub struct AltAZPostion {
    pub alt: f32,
    pub az: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct EqPostion {
    pub ra: f32,
    pub dec: f32,
}

impl Into<AltAZPostion> for EqPostion {
    fn into(self) -> AltAZPostion {
        // Placeholder conversion logic, should be replaced with actual conversion
        AltAZPostion {
            alt: self.dec,
            az: self.ra,
        }
    } 
}

impl Into<EqPostion> for AltAZPostion {
    fn into(self) -> EqPostion {
        // Placeholder conversion logic, should be replaced with actual conversion
        EqPostion {
            ra: self.az,
            dec: self.alt,
        }
    }  
}

#[derive(Debug, Clone, Copy)]
pub enum TelescopePosition {
    AltAz(AltAZPostion),
    Eq(EqPostion),
}

impl TelescopePosition {
    pub fn new_alt_az(alt: f32, az: f32) -> Self {
        TelescopePosition::AltAz(AltAZPostion { alt, az })
    }

    pub fn new_eq(ra: f32, dec: f32) -> Self {
        TelescopePosition::Eq(EqPostion { ra, dec })
    }
    
    pub fn get_alt_az(&self) -> AltAZPostion {
        match self {
            TelescopePosition::AltAz(pos) => pos.clone(),
            TelescopePosition::Eq(pos) => pos.clone().into(),
        }
    }

    pub fn get_eq(&self) -> EqPostion {
        match self {
            TelescopePosition::Eq(pos) => pos.clone(),
            TelescopePosition::AltAz(pos) => pos.clone().into(),
        }
    }
}