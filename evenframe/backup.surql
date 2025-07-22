-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- FUNCTIONS
-- ------------------------------

DEFINE FUNCTION fn::calc_order_total($prices: array<float>, $quantities: array<int>, $discount: float) { RETURN array::fold(array::transpose([$prices, $quantities]), 0, |$a: any, $b: any| $a + math::product($b)) - $discount; } PERMISSIONS FULL;
DEFINE FUNCTION fn::is_absolute_string($str: string) -> bool { RETURN $str.matches(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/); } PERMISSIONS FULL;

