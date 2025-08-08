-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY '9LbwSZFgQ76QGMWGeX3s91H8gK1Dvr7rmhRSLaZEfi3StTATRClNKYpc0ZfIB9YLDnSB8bHGu8YYYD27axeDh0OroEJztHwY0HHSz3MSyNl2aTZ0BUjjHxLslZRwq1Ja' WITH ISSUER KEY '9LbwSZFgQ76QGMWGeX3s91H8gK1Dvr7rmhRSLaZEfi3StTATRClNKYpc0ZfIB9YLDnSB8bHGu8YYYD27axeDh0OroEJztHwY0HHSz3MSyNl2aTZ0BUjjHxLslZRwq1Ja' DURATION FOR TOKEN 15m, FOR SESSION 6h;

