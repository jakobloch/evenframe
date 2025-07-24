-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'GjANnzReybNd5YEV8ae2ESGd7z13NnhJiwWtJDRsGHmEXxpz7oVuKTWid1AaqUEtKXW8OMcj29NRJmc097IFeqLd6tCExL88gFYdblxKMS95uipDr52hMJSuDZ7BYwDw' WITH ISSUER KEY 'GjANnzReybNd5YEV8ae2ESGd7z13NnhJiwWtJDRsGHmEXxpz7oVuKTWid1AaqUEtKXW8OMcj29NRJmc097IFeqLd6tCExL88gFYdblxKMS95uipDr52hMJSuDZ7BYwDw' DURATION FOR TOKEN 15m, FOR SESSION 6h;

