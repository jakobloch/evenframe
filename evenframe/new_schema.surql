-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'ZqayiXO6ZJZznxmNw5wMGBEyPKxvgfP4Dz3yFppjaWlAUXquZ8ZWnGiA1F32jA3pDTSOuX2rCgw9HQjG87AW6I1jTJFy9w8T4gJ4MeeZ7DH5f4cxMUl473ar6vM0w2IQ' WITH ISSUER KEY 'ZqayiXO6ZJZznxmNw5wMGBEyPKxvgfP4Dz3yFppjaWlAUXquZ8ZWnGiA1F32jA3pDTSOuX2rCgw9HQjG87AW6I1jTJFy9w8T4gJ4MeeZ7DH5f4cxMUl473ar6vM0w2IQ' DURATION FOR TOKEN 15m, FOR SESSION 6h;

