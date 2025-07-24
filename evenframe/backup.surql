-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'GjANnzReybNd5YEV8ae2ESGd7z13NnhJiwWtJDRsGHmEXxpz7oVuKTWid1AaqUEtKXW8OMcj29NRJmc097IFeqLd6tCExL88gFYdblxKMS95uipDr52hMJSuDZ7BYwDw' WITH ISSUER KEY 'GjANnzReybNd5YEV8ae2ESGd7z13NnhJiwWtJDRsGHmEXxpz7oVuKTWid1AaqUEtKXW8OMcj29NRJmc097IFeqLd6tCExL88gFYdblxKMS95uipDr52hMJSuDZ7BYwDw' DURATION FOR TOKEN 15m, FOR SESSION 6h;

-- ------------------------------
-- FUNCTIONS
-- ------------------------------

DEFINE FUNCTION fn::calc_order_total($prices: array<float>, $quantities: array<int>, $discount: float) { RETURN array::fold(array::transpose([$prices, $quantities]), 0, |$a: any, $b: any| $a + math::product($b)) - $discount; } PERMISSIONS FULL;
DEFINE FUNCTION fn::is_absolute_string($str: string) -> bool { RETURN $str.matches(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/); } PERMISSIONS FULL;

