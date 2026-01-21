#[macro_export]
macro_rules! define_tunable_params {
    (
        $(#[$outer:meta])*
        pub struct $name:ident {
            $(
            $(#[$inner:meta])*
            pub $field:ident : $type:ty
            ),* $(,)?
        }
    ) => {
        $(#[$outer])*
        pub struct $name {
            $(
            $(#[$inner])*
            pub $field : $type,
            )*
        }

        impl $name {
            pub fn to_vector(&self) -> Vec<f64> {
                let mut vec = Vec::with_capacity(SPSA_VECTOR_SIZE);

                $(
                    self.$field.push_to_vector(&mut vec);
                )*

                vec
            }

            pub fn from_vector(vec: &[f64]) -> Self {
                let mut idx = 0;

                Self {
                    $(
                       $field: <$type>::read_from_vector(vec, &mut idx),
                    )*
                }
            }
        }
    };
}
