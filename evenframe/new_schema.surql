-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'ssjUkgvZ9p7sbhEZHu0BnPa5K3wxzXKdZZzxdk26nHdjPg7YCzBDUQBvqFKNHdc1Mie2Jv6ZsHue9FLzxluOWbcT2nElgoH0MHNMsnjGBejks6mjj52tfl4kz12c8804' WITH ISSUER KEY 'ssjUkgvZ9p7sbhEZHu0BnPa5K3wxzXKdZZzxdk26nHdjPg7YCzBDUQBvqFKNHdc1Mie2Jv6ZsHue9FLzxluOWbcT2nElgoH0MHNMsnjGBejks6mjj52tfl4kz12c8804' DURATION FOR TOKEN 15m, FOR SESSION 6h;

