-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'g2EEyCA8lYkzQwliYRj6H6sZOi6VygMal9j4GLh7uUNpnYFMX5hgvtIL1x4GdxAkm2UoZLFF478ePUBvb6rk3eivl8j1UPI0UBEqM5iEAHPfLlJfGSy60tzaLIocX7Cj' WITH ISSUER KEY 'g2EEyCA8lYkzQwliYRj6H6sZOi6VygMal9j4GLh7uUNpnYFMX5hgvtIL1x4GdxAkm2UoZLFF478ePUBvb6rk3eivl8j1UPI0UBEqM5iEAHPfLlJfGSy60tzaLIocX7Cj' DURATION FOR TOKEN 15m, FOR SESSION 6h;

