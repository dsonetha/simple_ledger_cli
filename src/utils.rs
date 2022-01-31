use serde::Serializer;

pub fn round_serialize<S>(x: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let prec =  1e4;
    s.serialize_f64((x * prec).round() / prec)
}